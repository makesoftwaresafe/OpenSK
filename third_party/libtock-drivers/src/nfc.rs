use crate::util;
use core::cell::Cell;
use libtock_core::result::{ENOMEM, SUCCESS};
use libtock_core::{callback, syscalls};

const DRIVER_NUMBER: usize = 0x30003;

mod command_nr {
    pub const TRANSMIT: usize = 1;
    pub const RECEIVE: usize = 2;
    pub const EMULATE: usize = 3;
    pub const CONFIGURE: usize = 4;
    pub const FRAMEDELAYMAX: usize = 5;
}

mod subscribe_nr {
    pub const TRANSMIT: usize = 1;
    pub const RECEIVE: usize = 2;
    pub const SELECT: usize = 3;
}

mod allow_nr {
    pub const TRANSMIT: usize = 1;
    pub const RECEIVE: usize = 2;
}

pub enum TransOrRecvStatus {
    Error,
    InvalidBuffer,
    OOM,
    Success,
}

pub struct NfcTag {}

impl NfcTag {
    pub fn enable_emulation() -> bool {
        NfcTag::emulate(true)
    }

    pub fn disable_emulation() -> bool {
        NfcTag::emulate(false)
    }

    fn emulate(enabled: bool) -> bool {
        syscalls::command(DRIVER_NUMBER, command_nr::EMULATE, enabled as usize, 0).is_ok()
    }

    /// Subscribe to the tag being SELECTED callback.
    pub fn selected() -> bool {
        let is_selected = Cell::new(false);
        let mut is_selected_alarm = || is_selected.set(true);
        let subscription = syscalls::subscribe::<callback::Identity0Consumer, _>(
            DRIVER_NUMBER,
            subscribe_nr::SELECT,
            &mut is_selected_alarm,
        );
        if subscription.is_err() {
            return false;
        }

        util::yieldk_for(|| is_selected.get());
        true
    }

    /// Configure the tag type command.
    pub fn configure(tag_type: u8) -> bool {
        syscalls::command(DRIVER_NUMBER, command_nr::CONFIGURE, tag_type as usize, 0).is_ok()
    }

    /// Set the maximum frame delay value to support transmission with the reader.
    pub fn set_framedelaymax(delay: u32) -> bool {
        syscalls::command(DRIVER_NUMBER, command_nr::FRAMEDELAYMAX, delay as usize, 0).is_ok()
    }

    /// 1. Share with the driver a buffer.
    /// 2. Subscribe to having a successful receive callback.
    /// 3. Issue the request for reception.
    pub fn receive(buf: &mut [u8; 256]) -> TransOrRecvStatus {
        let result = syscalls::allow(DRIVER_NUMBER, allow_nr::RECEIVE, buf);
        if result.is_err() {
            return TransOrRecvStatus::InvalidBuffer;
        }

        let done = Cell::new(false);
        let mut alarm = || done.set(true);
        let subscription = syscalls::subscribe::<callback::Identity0Consumer, _>(
            DRIVER_NUMBER,
            subscribe_nr::RECEIVE,
            &mut alarm,
        );
        if subscription.is_err() {
            return TransOrRecvStatus::Error;
        }

        let result_code =
            unsafe { syscalls::raw::command(DRIVER_NUMBER, command_nr::RECEIVE, 0, 0) };
        match result_code {
            SUCCESS => (),
            ENOMEM => return TransOrRecvStatus::OOM,
            _ => return TransOrRecvStatus::Error,
        }

        util::yieldk_for(|| done.get());
        TransOrRecvStatus::Success
    }

    /// 1. Share with the driver a buffer containing the app's reply.
    /// 2. Subscribe to having a successful transmission callback.
    /// 3. Issue the request for transmitting.
    pub fn transmit(buf: &mut [u8], amount: usize) -> TransOrRecvStatus {
        let result = syscalls::allow(DRIVER_NUMBER, allow_nr::TRANSMIT, buf);
        if result.is_err() {
            return TransOrRecvStatus::InvalidBuffer;
        }

        let done = Cell::new(false);
        let mut alarm = || done.set(true);
        let subscription = syscalls::subscribe::<callback::Identity0Consumer, _>(
            DRIVER_NUMBER,
            subscribe_nr::TRANSMIT,
            &mut alarm,
        );
        if subscription.is_err() {
            return TransOrRecvStatus::Error;
        }

        let result_code = syscalls::command(DRIVER_NUMBER, command_nr::TRANSMIT, amount, 0);
        if result_code.is_err() {
            return TransOrRecvStatus::Error;
        }

        util::yieldk_for(|| done.get());
        TransOrRecvStatus::Success
    }
}
