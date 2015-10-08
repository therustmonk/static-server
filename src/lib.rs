//! Plugable HTTP Static server
//!
//! Features:
//!
//!  * Serve files from folder
//!  * Serve files from TAR-archive
//!  * Possible to write own files provider

#[macro_use]
extern crate log;
#[macro_use]
extern crate mime_guess;
extern crate hyper;
extern crate tar;

pub mod server;
pub mod provider;



