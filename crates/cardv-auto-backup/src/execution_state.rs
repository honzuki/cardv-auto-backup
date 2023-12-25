use std::marker::PhantomData;

use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_AWAYMODE_REQUIRED, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
    EXECUTION_STATE,
};

/// Execution state is a RAII wrapper for 'SetThreadExecutionState'
/// that automatically resets the state when dropped
#[derive(Debug)]
pub struct ExecutionState {
    _force_constructor_usage: PhantomData<()>,
}

impl ExecutionState {
    pub fn new(flags: EXECUTION_STATE) -> Self {
        unsafe { SetThreadExecutionState(flags) };

        Self {
            _force_constructor_usage: PhantomData,
        }
    }

    /// A shortcut of AWAYMODE & SYSTEM
    pub fn away_system() -> Self {
        Self::new(ES_CONTINUOUS | ES_AWAYMODE_REQUIRED | ES_SYSTEM_REQUIRED)
    }
}

impl Drop for ExecutionState {
    fn drop(&mut self) {
        // Safety: !
        unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
    }
}
