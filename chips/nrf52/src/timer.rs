//! nRF52 Timer
//! Completly copy-pasted from the implementation of nRF51
//! but the registers have been moved to peripheral_registers.rs
//! However, nRF52 comes with two additional timers that are not
//! supported at the moment


use chip;
use core::cell::Cell;
use core::mem;
use kernel::hil;
use nvic;
use peripheral_interrupts::NvicIdx;
use peripheral_registers;


#[derive(Copy,Clone)]
pub enum Location {
    TIMER0,
    TIMER1,
    TIMER2,
}

pub static mut TIMER0: Timer = Timer {
    which: Location::TIMER0,
    nvic: NvicIdx::TIMER0,
    client: Cell::new(None),
};

pub static mut ALARM1: TimerAlarm = TimerAlarm {
    which: Location::TIMER1,
    nvic: NvicIdx::TIMER1,
    client: Cell::new(None),
};

pub static mut TIMER2: Timer = Timer {
    which: Location::TIMER2,
    nvic: NvicIdx::TIMER2,
    client: Cell::new(None),
};

#[allow(non_snake_case)]
fn TIMER(location: Location) -> &'static peripheral_registers::TIMER {
    let ptr = peripheral_registers::TIMER_BASE +
              (location as usize) * peripheral_registers::TIMER_SIZE;
    unsafe { mem::transmute(ptr) }
}

pub trait CompareClient {
    /// Passes a bitmask of which of the 4 compares/captures fired (0x0-0xf).
    fn compare(&self, bitmask: u8);
}

pub struct Timer {
    which: Location,
    nvic: NvicIdx,
    client: Cell<Option<&'static CompareClient>>,
}

impl Timer {
    fn timer(&self) -> &'static peripheral_registers::TIMER {
        TIMER(self.which)
    }

    pub const fn new(location: Location, nvic: NvicIdx) -> Timer {
        Timer {
            which: location,
            nvic: nvic,
            client: Cell::new(None),
        }
    }

    pub fn set_client(&self, client: &'static CompareClient) {
        self.client.set(Some(client));
    }

    pub fn start(&self) {
        self.timer().task_start.set(1);
    }
    // Stops the timer and keeps the value
    pub fn stop(&self) {
        self.timer().task_stop.set(1);
    }
    // Stops the timer and clears the value
    pub fn shutdown(&self) {
        self.timer().task_shutdown.set(1);
    }
    // Clear the value
    pub fn clear(&self) {
        self.timer().task_clear.set(1);
    }

    /// Capture the current timer value into the CC register
    /// specified by which, and return the value.
    pub fn capture(&self, which: u8) -> u32 {
        match which {
            0 => {
                self.timer().task_capture[0].set(1);
                self.timer().cc[0].get()
            }
            1 => {
                self.timer().task_capture[1].set(1);
                self.timer().cc[1].get()
            }
            2 => {
                self.timer().task_capture[2].set(1);
                self.timer().cc[2].get()
            }
            _ => {
                self.timer().task_capture[3].set(1);
                self.timer().cc[3].get()
            }
        }
    }

    /// Capture the current value to the CC register specified by
    /// which and do not return the value.
    pub fn capture_to(&self, which: u8) {
        let _ = self.capture(which);
    }

    /// Shortcuts can automatically stop or clear the timer on a particular
    /// compare event; refer to section 18.3 of the nRF reference manual
    /// for details. Implementation currently provides shortcuts as the
    /// raw bitmask.
    pub fn get_shortcuts(&self) -> u32 {
        self.timer().shorts.get()
    }
    pub fn set_shortcuts(&self, shortcut: u32) {
        self.timer().shorts.set(shortcut);
    }

    pub fn get_cc0(&self) -> u32 {
        self.timer().cc[0].get()
    }
    pub fn set_cc0(&self, val: u32) {
        self.timer().cc[0].set(val);
    }
    pub fn get_cc1(&self) -> u32 {
        self.timer().cc[1].get()
    }
    pub fn set_cc1(&self, val: u32) {
        self.timer().cc[0].set(val);
    }
    pub fn get_cc2(&self) -> u32 {
        self.timer().cc[2].get()
    }
    pub fn set_cc2(&self, val: u32) {
        self.timer().cc[0].set(val);
    }
    pub fn get_cc3(&self) -> u32 {
        self.timer().cc[3].get()
    }
    pub fn set_cc3(&self, val: u32) {
        self.timer().cc[0].set(val);
    }

    pub fn enable_interrupts(&self, interrupts: u32) {
        self.timer().intenset.set(interrupts << 16);
    }
    pub fn disable_interrupts(&self, interrupts: u32) {
        self.timer().intenclr.set(interrupts << 16);
    }

    pub fn enable_nvic(&self) {
        nvic::enable(self.nvic);
    }

    pub fn disable_nvic(&self) {
        nvic::disable(self.nvic);
    }

    pub fn set_prescaler(&self, val: u8) {
        // Only bottom 4 bits are valid, so mask them
        // nRF51822 reference manual, page 102
        self.timer().prescaler.set((val & 0xf) as u32);
    }
    pub fn get_prescaler(&self) -> u8 {
        self.timer().prescaler.get() as u8
    }

    /// When an interrupt occurs, check if any of the 4 compares have
    /// created an event, and if so, add it to the bitmask of triggered
    /// events that is passed to the client.

    pub fn handle_interrupt(&self) {
        nvic::clear_pending(self.nvic);
        self.client.get().map(|client| {
            let mut val = 0;
            // For each of 4 possible compare events, if it's happened,
            // clear it and store its bit in val to pass in callback.
            for i in 0..4 {
                if self.timer().event_compare[i].get() != 0 {
                    val = val | 1 << i;
                    self.timer().event_compare[i].set(0);
                    self.disable_interrupts(1 << (i + 16));
                }
            }
            client.compare(val as u8);
        });
    }
}

pub struct TimerAlarm {
    which: Location,
    nvic: NvicIdx,
    client: Cell<Option<&'static hil::time::Client>>,
}

// CC0 is used for capture
// CC1 is used for compare/interrupts
const ALARM_CAPTURE: usize = 0;
const ALARM_COMPARE: usize = 1;
const ALARM_INTERRUPT_BIT: u32 = 1 << (16 + ALARM_COMPARE);

impl TimerAlarm {
    fn timer(&self) -> &'static peripheral_registers::TIMER {
        TIMER(self.which)
    }

    pub const fn new(location: Location, nvic: NvicIdx) -> TimerAlarm {
        TimerAlarm {
            which: location,
            nvic: nvic,
            client: Cell::new(None),
        }
    }

    pub fn clear(&self) {
        self.clear_alarm();
        self.timer().task_clear.set(1);
    }

    pub fn clear_alarm(&self) {
        self.timer().event_compare[ALARM_COMPARE].set(0);
        self.disable_interrupts();
        nvic::clear_pending(self.nvic);
    }

    pub fn set_client(&self, client: &'static hil::time::Client) {
        self.client.set(Some(client));
    }

    pub fn start(&self) {
        // Make timer 32 bits wide
        self.timer().bitmode.set(3);
        // Clock is 16MHz, so scale down by 2^10 to 16KHz
        self.timer().prescaler.set(10);
        self.timer().task_start.set(1);
    }

    pub fn stop(&self) {
        self.timer().task_stop.set(1);
    }

    pub fn handle_interrupt(&self) {
        self.clear_alarm();
        self.client.get().map(|client| { client.fired(); });
    }

    // Enable and disable interrupts use the bottom 4 bits
    // for the 4 compare interrupts. These functions shift
    // those bits to the correct place in the register.
    pub fn enable_interrupts(&self) {
        self.timer().intenset.set(ALARM_INTERRUPT_BIT);
    }

    pub fn disable_interrupts(&self) {
        self.timer().intenclr.set(ALARM_INTERRUPT_BIT);
    }

    pub fn interrupts_enabled(&self) -> bool {
        self.timer().intenset.get() == (ALARM_INTERRUPT_BIT)
    }

    pub fn enable_nvic(&self) {
        nvic::enable(self.nvic);
    }

    pub fn disable_nvic(&self) {
        nvic::disable(self.nvic);
    }

    pub fn value(&self) -> u32 {
        self.timer().task_capture[ALARM_CAPTURE].set(1);
        self.timer().cc[ALARM_CAPTURE].get()
    }
}

impl hil::time::Time for TimerAlarm {
    type Frequency = hil::time::Freq16KHz;

    fn disable(&self) {
        self.disable_interrupts();
    }

    fn is_armed(&self) -> bool {
        self.interrupts_enabled()
    }
}

impl hil::time::Alarm for TimerAlarm {
    fn now(&self) -> u32 {
        self.value()
    }

    fn set_alarm(&self, tics: u32) {
        self.disable_interrupts();
        self.timer().cc[ALARM_COMPARE].set(tics);
        self.clear_alarm();
        self.enable_interrupts();
    }

    fn get_alarm(&self) -> u32 {
        self.timer().cc[ALARM_COMPARE].get()
    }
}


#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn TIMER0_Handler() {
    use kernel::common::Queue;

    nvic::disable(NvicIdx::TIMER0);
    chip::INTERRUPT_QUEUE.as_mut().unwrap().enqueue(NvicIdx::TIMER0);
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn TIMER1_Handler() {
    use kernel::common::Queue;

    nvic::disable(NvicIdx::TIMER1);
    chip::INTERRUPT_QUEUE.as_mut().unwrap().enqueue(NvicIdx::TIMER1);
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn TIMER2_Handler() {
    use kernel::common::Queue;

    nvic::disable(NvicIdx::TIMER2);
    chip::INTERRUPT_QUEUE.as_mut().unwrap().enqueue(NvicIdx::TIMER2);
}
