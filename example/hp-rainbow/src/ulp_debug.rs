// Non-public debugging interfaces for the RISCV ULP,
// figured out by inspection of PicoRV32 source code,
// and reading forums - Leigh Oliver
// Originally written for stompy-ulp/hp-core project,
// on the 2nd of January 2026.
#![allow(dead_code)]
use log::info;
use esp_hal::peripherals;

pub trait FromRegister {
    fn read() -> Self;
}

#[derive(Debug)]
#[allow(unused)]
pub struct SarCocpuState {
    clk_en_st: bool,
    reset_n: bool,
    eoi: bool,
    trap: bool,
    ebreak: bool
}

impl FromRegister for SarCocpuState {
    #[cfg(esp32s3)]
    fn read() -> Self {
        let r = unsafe { &*peripherals::SENS::PTR }.sar_cocpu_state().read();
        SarCocpuState {
            clk_en_st: r.sar_cocpu_clk_en_st().bit_is_set(),
            reset_n: r.sar_cocpu_reset_n().bit_is_set(),
            eoi: r.sar_cocpu_eoi().bit_is_set(),
            trap: r.sar_cocpu_trap().bit_is_set(),
            ebreak: r.sar_cocpu_ebreak().bit_is_set(),
        }
    }
    #[cfg(esp32s2)]
    fn read() -> Self {
        let r = unsafe { &*peripherals::SENS::PTR }.sar_cocpu_state().read();
        SarCocpuState {
            clk_en_st: r.cocpu_clk_en().bit_is_set(),
            reset_n: r.cocpu_reset_n().bit_is_set(),
            eoi: r.cocpu_eoi().bit_is_set(),
            trap: r.cocpu_trap().bit_is_set(),
            ebreak: r.cocpu_ebreak().bit_is_set(),
        }
    }
}


#[derive(Debug)]
#[allow(unused)]
pub struct CocpuDebug {
    pc: u16,
    mem_valid: bool,
    mem_ready: bool,
    write_enable: u8,
    mem_address: u16,
    state : SarCocpuState
}

impl CocpuDebug {
    fn trigger_debug() {
        #[cfg(esp32s3)]
        unsafe {{ &*peripherals::SENS::PTR }.sar_cocpu_state().write(|w| w.sar_cocpu_dbg_trigger().set_bit())};
        #[cfg(esp32s2)]
        unsafe {{ &*peripherals::SENS::PTR }.sar_cocpu_state().write(|w| w.cocpu_dbg_trigger().set_bit())};
    }
    fn read_debug() -> Self {
        let r = unsafe { &*peripherals::SENS::PTR }.sar_cocpu_debug().read();

        #[cfg(esp32s3)]
        return CocpuDebug {
            pc: r.sar_cocpu_pc().bits(),
            mem_valid: r.sar_cocpu_mem_vld().bit_is_set(),
            mem_ready: r.sar_cocpu_mem_rdy().bit_is_set(),
            write_enable: r.sar_cocpu_mem_wen().bits(),
            mem_address: r.sar_cocpu_mem_addr().bits(),
            state: SarCocpuState::read(),
        };

        #[cfg(esp32s2)]
        return CocpuDebug {
            pc: r.cocpu_pc().bits(),
            mem_valid: r.cocpu_mem_vld().bit_is_set(),
            mem_ready: r.cocpu_mem_rdy().bit_is_set(),
            write_enable: r.cocpu_mem_wen().bits(),
            mem_address: r.cocpu_mem_addr().bits(),
            state: SarCocpuState::read(),
        };
    }
}

impl FromRegister for CocpuDebug {
    fn read() -> Self {
        // Forum quote:
        // "SENS_SAR_COCPU_DEBUG_REG might be helpful. Set SENS_SAR_COCPU_STATE_REG[SENS_COCPU_DBG_TRIGGER] to update (I'm not sure it's immediate though?)."
        // 1. Trigger the debug prompt
        CocpuDebug::trigger_debug();
        // 2. Read the debug register
        CocpuDebug::read_debug()
    }
}

pub fn dump_coproc_pc_instructions(dbg: CocpuDebug) {
    // Using the 'pc' field of CocpuDebug,
    // reads the data from RTC_SLOW_MEM and prints it as hex.
    // Will print an instruction before and after this too.
    let pc = (dbg.pc as u32 + 0x50000000) as *mut u32;
    let instr = unsafe { pc.read_unaligned() };
    info!("*PC({:04x}): {:08x}", dbg.pc, instr);
}