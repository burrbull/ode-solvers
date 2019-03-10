//===================================================================//
// Copyright (c) 2018, Sylvain Renevey
// Copyright (c) 2004, Ernst Hairer

// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:

// - Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.

// - Redistributions in binary form must reproduce the above copyright
// notice, this list of conditions and the following disclaimer in the
// documentation and/or other materials provided with the distribution.

// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS “AS
// IS” AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
// TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
// PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE REGENTS OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

// Written by:
//      Sylvain Renevey (syl.renevey@gmail.com)
//
// This code is a Rust adaptation of the code written originally
// in Fortran by:
//
//      E. Hairer & G. Wanner
//      Université de Genève, dept. de Mathématiques
//      CH-1211 Genève 4, Swizerland
//      E-mail : hairer@divsun.unige.ch, wanner@divsun.unige.ch
//
// and adapted for C by:
//      J.Colinge (colinge@divsun.unige.ch).
//
// and C++ by:
//      Blake Ashby (bmashby@stanford.edu)
//
//===================================================================//

#![allow(clippy::needless_range_loop, clippy::unreadable_literal)]

//! Explicit Runge-Kutta method with Dormand-Prince coefficients of order 5(4) and dense output of order 4.

use crate::butcher_tableau::Dopri54;
use crate::controller::Controller;
use crate::dop_shared::*;
use std::f64;

type V = ndarray::Array1<f64>;
type W = ndarray::Array2<f64>;

trait DefaultController {
    fn default(x: f64, x_end: f64) -> Self;
}

impl DefaultController for Controller {
    fn default(x: f64, x_end: f64) -> Self {
        let alpha = 0.2 - 0.04 * 0.75;
        Controller::new(alpha, 0.04, 10.0, 0.2, x_end - x, 0.9, sign(1.0, x_end - x))
    }
}

/// Structure containing the parameters for the numerical integration.
pub struct Dopri5<F: System> {
    f: F,
    x: f64,
    x_old: f64,
    x_end: f64,
    xd: f64,
    dx: f64,
    y: V,
    rtol: f64,
    atol: f64,
    x_out: Vec<f64>,
    y_out: Vec<V>,
    uround: f64,
    h: f64,
    h_old: f64,
    n_max: u32,
    n_stiff: u32,
    coeffs: Dopri54,
    controller: Controller,
    out_type: OutputType,
    rcont: W,
    stats: Stats,
    solout: fn(f64, &V, &V) -> bool,
}

impl<F: System> Dopri5<F> {
    /// Default initializer for the structure
    ///
    /// # Arguments
    ///
    /// * `f`       - Pointer to the function to integrate
    /// * `x`       - Initial value of the independent variable (usually time)
    /// * `x_end`   - Final value of the independent variable
    /// * `dx`      - Increment in the dense output. This argument has no effect if the output type is Sparse
    /// * `y`       - Initial value of the dependent variable(s)
    /// * `rtol`    - Relative tolerance used in the computation of the adaptive step size
    /// * `atol`    - Absolute tolerance used in the computation of the adaptive step size
    ///
    pub fn new(
        f: F,
        x: f64,
        x_end: f64,
        dx: f64,
        y: V,
        rtol: f64,
        atol: f64,
    ) -> Self {
        Self {
            f,
            x,
            xd: x,
            dx,
            x_old: x,
            x_end,
            y,
            rtol,
            atol,
            x_out: Vec::new(),
            y_out: Vec::new(),
            uround: f64::EPSILON,
            h: 0.0,
            h_old: 0.0,
            n_max: 100000,
            n_stiff: 1000,
            coeffs: Dopri54::new(),
            controller: Controller::default(x, x_end),
            out_type: OutputType::Dense,
            rcont: W::zeros((5, F::DIM)),
            stats: Stats::new(),
            solout: |_, _, _| false,
        }
    }

    /// Advanced initializer for the structure.
    ///
    /// # Arguments
    ///
    /// * `f`       - Pointer to the function to integrate
    /// * `x`       - Initial value of the independent variable (usually time)
    /// * `x_end`   - Final value of the independent variable
    /// * `dx`      - Increment in the dense output. This argument has no effect if the output type is Sparse
    /// * `y`       - Initial value of the dependent variable(s)
    /// * `rtol`    - Relative tolerance used in the computation of the adaptive step size
    /// * `atol`    - Absolute tolerance used in the computation of the adaptive step size
    /// * `safety_factor`   - Safety factor used in the computation of the adaptive step size
    /// * `beta`    - Value of the beta coefficient of the PI controller. Default is 0.04
    /// * `fac_min` - Minimum factor between two successive steps. Default is 0.2
    /// * `fac_max` - Maximum factor between two successive steps. Default is 10.0
    /// * `h_max`   - Maximum step size. Default is `x_end-x`
    /// * `h`       - Initial value of the step size. If h = 0.0, the intial value of h is computed automatically
    /// * `n_max`   - Maximum number of iterations. Default is 100000
    /// * `n_stiff` - Stifness is tested when the number of iterations is a multiple of n_stiff. Default is 1000
    /// * `out_type`    - Type of the output. Must be a variant of the OutputType enum. Default is Dense
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn from_param(
        f: F,
        x: f64,
        x_end: f64,
        dx: f64,
        y: V,
        rtol: f64,
        atol: f64,
        safety_factor: f64,
        beta: f64,
        fac_min: f64,
        fac_max: f64,
        h_max: f64,
        h: f64,
        n_max: u32,
        n_stiff: u32,
        out_type: OutputType,
    ) -> Self {
        let alpha = 0.2 - beta * 0.75;
        Self {
            f,
            x,
            xd: x,
            x_old: 0.,
            x_end,
            dx,
            y,
            rtol,
            atol,
            x_out: Vec::new(),
            y_out: Vec::new(),
            uround: f64::EPSILON,
            h,
            h_old: 0.,
            n_max,
            n_stiff,
            coeffs: Dopri54::new(),
            controller: Controller::new(
                alpha,
                beta,
                fac_max,
                fac_min,
                h_max,
                safety_factor,
                sign(1., x_end - x),
            ),
            out_type,
            rcont: W::zeros((5, F::DIM)),
            stats: Stats::new(),
            solout: |_, _, _| false,
        }
    }

    /// Compute the initial stepsize
    fn hinit(&self) -> f64 {
        let mut f0 = V::zeros(F::DIM);
        self.f.system(self.x, &self.y, &mut f0);
        let posneg = sign(1.0, self.x_end - self.x);

        // Compute the norm of y0 and f0
        let mut d0 = 0.0;
        let mut d1 = 0.0;
        for i in 0..F::DIM {
            let y_i: f64 = self.y[i];
            let sci: f64 = self.atol + y_i.abs() * self.rtol;
            d0 += (y_i / sci) * (y_i / sci);
            let f0_i: f64 = f0[i];
            d1 += (f0_i / sci) * (f0_i / sci);
        }

        // Compute h0
        let mut h0 = if d0 < 1_e-10 || d1 < 1_e-10 {
            1_e-6
        } else {
            0.01 * (d0 / d1).sqrt()
        };

        h0 = h0.min(self.controller.h_max());
        h0 = sign(h0, posneg);

        let y1 = f0.to_owned() * h0 + &self.y;
        let mut f1 = V::zeros(F::DIM);
        self.f.system(self.x + h0, &y1, &mut f1);

        // Compute the norm of f1-f0 divided by h0
        let mut d2: f64 = 0.;
        for i in 0..F::DIM {
            let f0_i: f64 = f0[i];
            let f1_i: f64 = f1[i];
            let y_i: f64 = self.y[i];
            let sci: f64 = self.atol + y_i.abs() * self.rtol;
            d2 += ((f1_i - f0_i) / sci) * ((f1_i - f0_i) / sci);
        }
        d2 = d2.sqrt() / h0;

        let h1 = if d1.sqrt().max(d2.abs()) <= 1_e-15 {
            (1_e-6 as f64).max(h0.abs() * 1_e-3)
        } else {
            (0.01 / (d1.sqrt().max(d2))).powf(1. / 5.)
        };

        sign(
            (100. * h0.abs()).min(h1.min(self.controller.h_max())),
            posneg,
        )
    }

    /// Set stop function will be called at every successful integration step.
    pub fn set_solout(&mut self, solout: fn(f64, &V, &V) -> bool) {
        self.solout = solout;
    }

    /// Core integration method.
    pub fn integrate(&mut self) -> Result<Stats, IntegrationError> {
        // Initilization
        self.x_old = self.x;
        let mut n_step = 0;
        let mut last = false;
        let mut h_new = 0.;
        let mut iter_non_stiff = 1..7;
        let mut iter_iasti = 1..16;
        let posneg = sign(1., self.x_end - self.x);

        if self.h == 0. {
            self.h = self.hinit();
            self.stats.num_eval += 2;
        }
        self.h_old = self.h;

        // Save initial values
        if self.out_type == OutputType::Sparse {
            self.x_out.push(self.x);
            self.y_out.push(self.y.to_owned());
        }

        let mut k = vec![V::zeros(F::DIM); 7];
        self.f.system(self.x, &self.y, &mut k[0]);
        self.stats.num_eval += 1;

        // Main loop
        while !last {
            // Check if step number is within allowed range
            if n_step > self.n_max {
                self.h_old = self.h;
                return Err(IntegrationError::MaxNumStepReached { x: self.x, n_step });
            }

            // Check for step size underflow
            if 0.1 * self.h.abs() <= self.uround * self.x.abs() {
                self.h_old = self.h;
                return Err(IntegrationError::StepSizeUnderflow { x: self.x });
            }

            // Check if it's the last iteration
            if (self.x + 1.01 * self.h - self.x_end) * posneg > 0. {
                self.h = self.x_end - self.x;
                last = true;
            }
            n_step += 1;

            // 6 Stages
            let mut y_next = V::zeros(F::DIM);
            let mut y_stiff = V::zeros(F::DIM);
            for s in 1..7 {
                y_next.assign(&self.y);
                for j in 0..s {
                    y_next += &(k[j].to_owned() * (self.h * self.coeffs.a(s + 1, j + 1)));
                }
                self.f.system(self.x + self.h * self.coeffs.c(s + 1), &y_next, &mut k[s]);
                if s == 5 {
                    y_stiff.assign(&y_next);
                }
            }
            k[1] = k[6].to_owned();
            self.stats.num_eval += 6;

            // Prepare dense output
            if self.out_type == OutputType::Dense {
                self.rcont.row_mut(0).assign(
                    &((k[0].to_owned() * self.coeffs.d(1)
                        + k[2].to_owned() * self.coeffs.d(3)
                        + k[3].to_owned() * self.coeffs.d(4)
                        + k[4].to_owned() * self.coeffs.d(5)
                        + k[5].to_owned() * self.coeffs.d(6)
                        + k[1].to_owned() * self.coeffs.d(7))
                        * self.h),
                );
            }

            // Compute error estimate
            k[3] = (k[0].to_owned() * self.coeffs.e(1)
                + k[1].to_owned() * self.coeffs.e(2)
                + k[2].to_owned() * self.coeffs.e(3)
                + k[3].to_owned() * self.coeffs.e(4)
                + k[4].to_owned() * self.coeffs.e(5)
                + k[5].to_owned() * self.coeffs.e(6)
                + k[1].to_owned() * self.coeffs.e(7))
                * self.h;

            // Compute error
            let mut err = 0.0;
            for i in 0..F::DIM {
                let y_i: f64 = self.y[i];
                let y_next_i: f64 = y_next[i];
                let sc_i: f64 = self.atol + y_i.abs().max(y_next_i.abs()) * self.rtol;
                let err_est_i: f64 = k[3][i];
                err += (err_est_i / sc_i) * (err_est_i / sc_i);
            }
            err = (err / F::DIM as f64).sqrt();

            // Step size control
            if self.controller.accept(err, self.h, &mut h_new) {
                self.stats.accepted_steps += 1;

                // Stifness detection
                if (self.stats.accepted_steps % self.n_stiff != 0) || F::DIM > 0 {
                    let kd = k[1].to_owned() - &k[5];
                    let num: f64 = kd.dot(&kd);
                    let yd = y_next.to_owned() - &y_stiff;
                    let den: f64 = yd.dot(&yd);
                    let h_lamb = if den > 0.0 {
                        self.h * (num / den).sqrt()
                    } else {
                        0.0
                    };

                    if h_lamb > 3.25 {
                        iter_non_stiff = 1..7;
                        if iter_iasti.next() == Some(15) {
                            self.h_old = self.h;
                            return Err(IntegrationError::StiffnessDetected { x: self.x });
                        }
                    } else if iter_non_stiff.next() == Some(6) {
                        iter_iasti = 1..16;
                    }
                }

                // Prepare dense output
                if self.out_type == OutputType::Dense {
                    let ydiff = y_next.to_owned() - &self.y;
                    let bspl = k[0].to_owned() * self.h - &ydiff;
                    self.rcont.row_mut(0).assign(&self.y);
                    self.rcont.row_mut(1).assign(&ydiff);
                    self.rcont.row_mut(2).assign(&bspl);
                    self.rcont
                        .row_mut(3)
                        .assign(&(-k[1].to_owned() * self.h + &ydiff - bspl));
                }

                k[0] = k[1].to_owned();
                self.y = y_next.to_owned();
                self.x_old = self.x;
                self.x += self.h;
                self.h_old = self.h;

                self.solution_output(y_next.to_owned());

                if (self.solout)(self.x, &self.y, &k[0]) {
                    last = true;
                }

                // Normal exit
                if last {
                    self.h_old = posneg * h_new;
                    return Ok(self.stats);
                }
            } else if self.stats.accepted_steps >= 1 {
                self.stats.rejected_steps += 1;
            }
            self.h = h_new;
        }
        Ok(self.stats)
    }

    fn solution_output(&mut self, y_next: V) {
        if self.out_type == OutputType::Dense {
            while self.xd.abs() <= self.x.abs() {
                if self.x_old.abs() <= self.xd.abs() && self.x.abs() >= self.xd.abs() {
                    let theta = (self.xd - self.x_old) / self.h_old;
                    let theta1 = 1.0 - theta;
                    self.x_out.push(self.xd);
                    self.y_out.push(
                        (((self.rcont.row(4).to_owned() * theta1 + self.rcont.row(3)) * theta
                            + self.rcont.row(2))
                            * theta1
                            + self.rcont.row(1))
                            * theta
                            + self.rcont.row(0),
                    );
                    self.xd += self.dx;
                }
            }
        } else {
            self.x_out.push(self.x);
            self.y_out.push(y_next);
        }
    }

    /// Getter for the independent variable's output.
    pub fn x_out(&self) -> &Vec<f64> {
        &self.x_out
    }

    /// Getter for the dependent variables' output.
    pub fn y_out(&self) -> &Vec<V> {
        &self.y_out
    }
}

fn sign(a: f64, b: f64) -> f64 {
    if b > 0.0 {
        a.abs()
    } else {
        -a.abs()
    }
}