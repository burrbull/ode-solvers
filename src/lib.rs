//! # ODEs Solvers
//! `ode-solvers` is a collection of numerical methods to solve ordinary differential equations (ODEs).

use ndarray as nd;

// Declare modules
pub mod butcher_tableau;
pub mod controller;
//pub mod dop853;
pub mod dopri5;
pub mod dop_shared;
