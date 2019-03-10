// Chemical reaction of Robertson.
// This ode is stiff and is used to test the automatic stiffness detection in dopri5 and/or dop853.

use ndarray as nd;
use ode_solvers::dop853::*;

type State = nd::Array1<f64>;
type Time = f64;

fn main() {
    let y0 = nd::arr1(&[1., 0., 0.]);
    let solver = Solver;
    let mut stepper = Dop853::new(solver, 0., 0.3, 0.3, y0, 1_e-2, 1_e-6);
    let res = stepper.integrate();

    // Handle result
    match res {
        Ok(stats) => println!("{}", stats),
        Err(e) => println!("An error occured: {}", e),
    }
}

struct Solver;

impl ode_solvers::System for Solver {
    const DIM: usize = 3;
    fn system(&self, _: Time, y: &State, dy: &mut State) {
        dy[0] = -0.04 * y[0] + 10000. * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 10000. * y[1] * y[2] - 3. * (10. as f64).powi(7) * y[1] * y[1];
        dy[2] = 3. * (10. as f64).powi(7) * y[1] * y[1];
    }
}