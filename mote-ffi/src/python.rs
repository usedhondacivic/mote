//! Foreign function interface for Python

#[pyo3::pymodule]
mod mote_ffi {
    use pyo3::{exceptions::PyIOError, prelude::*};
    use std::string::{String, ToString};

    use crate::{Error, MoteCommsFFI};
    use mote_api::{
        MoteLink,
        messages::{host_to_mote, mote_to_host},
    };

    impl std::convert::From<Error> for PyErr {
        fn from(err: Error) -> PyErr {
            PyIOError::new_err(err.to_string())
        }
    }

    #[pyclass]
    struct Link {
        link: MoteCommsFFI<1400, mote_to_host::Message, host_to_mote::Message>,
    }

    #[pymethods]
    impl Link {
        #[new]
        fn new() -> Self {
            Self {
                link: MoteCommsFFI::from(MoteLink::new()),
            }
        }

        fn send(&mut self, message: String) -> Result<(), Error> {
            self.link.send(&message)?;
            Ok(())
        }

        fn poll_transmit(&mut self) -> Result<Option<String>, Error> {
            self.link.poll_transmit()
        }

        fn handle_receive(&mut self, packet: String) -> Result<(), Error> {
            self.link.handle_receive(&packet)
        }

        fn poll_receive(&mut self) -> Result<Option<String>, Error> {
            self.link.poll_receive()
        }
    }
}
