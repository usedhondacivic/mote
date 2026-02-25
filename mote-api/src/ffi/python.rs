//! Foreign function interface for Python

#[pyo3::pymodule]
mod mote_api {
    use pyo3::{exceptions::PyIOError, prelude::*};
    use std::string::{String, ToString};

    use crate::{MoteLink, ffi::Error};

    impl std::convert::From<Error> for PyErr {
        fn from(err: Error) -> PyErr {
            PyIOError::new_err(err.to_string())
        }
    }

    #[pyclass]
    struct Link {
        link: MoteLink,
    }

    #[pymethods]
    impl Link {
        #[new]
        fn new() -> Self {
            Self {
                link: MoteLink::new(),
            }
        }

        fn send(&mut self, message: String) -> Result<(), Error> {
            self.link.send_json(&message)?;
            Ok(())
        }

        fn poll_transmit(&mut self) -> Result<Option<String>, Error> {
            self.link.poll_transmit_json()
        }

        fn handle_receive(&mut self, packet: String) -> Result<(), Error> {
            self.link.handle_receive_json(&packet)
        }

        fn poll_receive(&mut self) -> Result<Option<String>, Error> {
            self.link.poll_receive_json()
        }
    }
}
