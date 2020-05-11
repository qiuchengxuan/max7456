use peripheral_register::{register_fields, Field};

#[macro_export]
macro_rules! brightness {
    (0%) => {
        0
    };
    (7%) => {
        1
    };
    (14%) => {
        2
    };
    (21%) => {
        3
    };
    (28%) => {
        4
    };
    (35%) => {
        5
    };
    (42%) => {
        6
    };
    (49%) => {
        7
    };
}

pub enum SyncMode {
    AutoSyncDetect = 0,
    External = 0b10,
    Internal = 0b11,
}

pub enum VerticalSync {
    Immediately = 0,
    NextVSync = 1,
}

pub enum Standard {
    NTSC = 0,
    PAL = 1,
}

register_fields! {
    #[derive(Debug)]
    pub enum VideoMode0 {
        Standard = 6: 1,
        SyncMode = 4: 2,
        EnableDisplay = 3: 1,
        VerticalSync = 2: 1,
        SoftwareReset = 1: 1,
        VideoBufferEnable = 0: 1,
    }

    #[derive(Debug)]
    pub enum VideoMode1 {
        BackgroundMode = 7: 1,
        Brightness = 4: 3,
        BlinkTime = 2: 2,
        BlinkDutyCycle = 0: 2,
    }
}

register_fields! {
    #[derive(Debug)]
    enum HorizentalOffset {
        Offset = 0: 5, // unit pixel
    }

    #[derive(Debug)]
    enum VerticalOffset {
        Offset = 0: 4, // unit pixel
    }
}

pub enum OperationMode {
    Mode16Bit = 0, //
    Mode8Bit = 1,
}

register_fields! {
    #[derive(Debug)]
    pub enum DisplayMemoryMode {
        OperationMode = 6: 1,
        LocalBackgroundControl = 5: 1,
        Blink = 4: 1,
        Invert = 3: 1,
        Clear = 2: 1,
        VerticalSyncClear = 1: 1,
        AutoIncrement = 0: 1, // reset on ClearDisplayMemory set
    }
}

register_fields! {
    #[derive(Debug)]
    pub enum DisplayMemoryAddressHigh {
        ByteSelection = 1: 1,
        Address8 = 0: 1,
    }

    #[derive(Debug)]
    pub enum DisplayMemoryAddressLow {
        Address = 0: 7,
    }
}

// 30x16, use as DisplayMemoryAddress High and Low
#[inline]
pub fn display_memory_address(row: u8, column: u8) -> u16 {
    row as u16 * 30 + column as u16
}

pub enum Pixel {
    Black = 0,
    Transparent = 1,
    White = 2,
}

register_fields! {
    #[derive(Debug)]
    enum CharacterMemoryData {
        Pixel0 = 6: 2, // left-most
        Pixel1 = 4: 2,
        Pixel2 = 2: 2,
        Pixel3 = 0: 2, // right-most
    }
}

#[macro_export]
macro_rules! rise_and_fall_time {
    (20ns) => {
        0
    };
    (30ns) => {
        1
    };
    (35ns) => {
        2
    };
    (60ns) => {
        3
    };
    (80ns) => {
        4
    };
    (100ns) => {
        5
    };
}

#[macro_export]
macro_rules! insertion_mux_switch_time {
    (30ns) => {
        0
    };
    (35ns) => {
        1
    };
    (50ns) => {
        2
    };
    (75ns) => {
        3
    };
    (100ns) => {
        4
    };
    (120ns) => {
        5
    };
}

register_fields! {
    #[derive(Debug)]
    pub enum OSDInsertionMuxRegister {
        RiseAndFallTime = 3: 3,
        InsertionMuxSwitchingTIme = 0: 3,
    }
}

pub enum CharacterMemoryStatus {
    Available = 0,
    Unavailable = 1,
}

register_fields! {
    #[derive(Debug)]
    pub enum Status {
        ResetMode = 6: 1,
        CharacterMemoryStatus = 5: 1,
        VSyncOutputLevel = 4: 1,
        HSyncOutputLevel = 3: 1,
        LossOfSync = 2: 1,
        NTSCSignal = 1: 1,
        PALSignal = 0: 1,
    }
}

pub enum CharacterMemoryMode {
    WriteToNVM = 0xA0,
    ReadFromNVM = 0x50,
}

#[derive(Copy, Clone)]
pub enum Registers {
    VideoMode0 = 0,
    VideoMode1 = 1,
    HorizentalOffset = 2,
    VerticalOffset = 3,
    DisplayMemoryMode = 4,
    DisplayMemoryAddressHigh = 5,
    DisplayMemoryAddressLow = 6,
    DisplayMemoryDataIn = 7,
    CharacterMemoryMode = 8,
    CharacterMemoryAddressHigh = 9,
    CharacterMemoryAddressLow = 0xA, // 0~5bit
    CharacterMemoryDataIn = 0xB,
    OSDInsertionMux = 0xC,
    Row0Bridghtness = 0x10,
    Status = 0xA0,
    DisplayMemoryDataOut = 0xB0,   // read only
    CharacterMemoryDataOut = 0xC0, // read only
}

impl Registers {
    pub fn read_address(&self) -> u8 {
        *self as u8 | 0x80
    }
}
