use ndarray as nd;
use ode_solvers::dop853::*;

// Define type aliases for the state and time types
type State = nd::Array1<f64>;
type Time = f64;

use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// Define problem specific constant
const MU: f64 = 0.012300118882173;

fn main() {
    let y0 = nd::arr1(&[-0.271, -0.42, 0.0, 0.3, -1.0, 0.0]);
    let mut stepper = Dop853::new(system, 0.0, 150.0, 0.002, y0, 1.0e-14, 1.0e-14);
    let res = stepper.integrate();

    // Handle result
    match res {
        Ok(stats) => {
            println!("{}", stats);
            let path = Path::new("./outputs/three_body_dop853.dat");
            save(stepper.x_out(), stepper.y_out(), path);
            println!("Results saved in: {:?}", path);
        }
        Err(e) => println!("An error occured: {}", e),
    }
}

fn system(_t: Time, y: &State, dy: &mut State) {
    let d = ((y[0] + MU).powi(2) + y[1].powi(2) + y[2].powi(2)).sqrt();
    let r = ((y[0] - 1.0 + MU).powi(2) + y[1].powi(2) + y[2].powi(2)).sqrt();

    dy[0] = y[3];
    dy[1] = y[4];
    dy[2] = y[5];
    dy[3] = y[0] + 2.0 * y[4]
        - (1.0 - MU) * (y[0] + MU) / d.powi(3)
        - MU * (y[0] - 1.0 + MU) / r.powi(3);
    dy[4] = -2.0 * y[3] + y[1] - (1.0 - MU) * y[1] / d.powi(3) - MU * y[1] / r.powi(3);
    dy[5] = -(1.0 - MU) * y[2] / d.powi(3) - MU * y[2] / r.powi(3);
}

pub fn save(times: &Vec<Time>, states: &Vec<State>, filename: &Path) {
    // Create or open file
    let mut buf = match File::create(filename) {
        Err(e) => {
            println!("Could not open file. Error: {:?}", e);
            return;
        }
        Ok(buf) => buf,
    };
    // Write time and state in a csv format
    for (i, state) in states.iter().enumerate() {
        buf.write_fmt(format_args!("{}", times[i])).unwrap();
        for val in state.iter() {
            buf.write_fmt(format_args!(", {}", val)).unwrap();
        }
        buf.write_fmt(format_args!("\n")).unwrap();
    }
}
