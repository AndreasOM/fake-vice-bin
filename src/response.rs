use std::collections::HashMap;

use crate::ResponseHeader;

pub enum Response {
	RegistersGet {
		registers: HashMap<u8, (u8, u16)>, // id -> size, value
	},
	RegistersAvailable {
		registers: HashMap<u8, (u8, String)>, // id -> size, name
	},
	Stopped {
		pc: u16,
	},
	Resumed {
		pc: u16,
	},
	AdvanceInstructions,
	Ping,
	Reset,
	Exit,
	Invalid,
}

impl From<(&ResponseHeader, &[u8])> for Response {
	fn from(parts: (&ResponseHeader, &[u8])) -> Self {
		let rh = parts.0;
		let buffer = parts.1;
		match rh.response_type() {
			0x31 => {
				// registers get
				let c = [buffer[1], buffer[0]];
				let mut count = 0;
				for b in c.iter() {
					count <<= 8;
					count |= *b as u16;
				}
				//println!("Count {}", count);
				let mut entry_start = 2;
				let mut registers = HashMap::new();
				for e in 0..count {
					let size = buffer[entry_start];
					let id = buffer[entry_start + 1];
					let value =
						((buffer[entry_start + 3] as u16) << 8) | (buffer[entry_start + 2] as u16);

					let r = (size, value);
					registers.insert(id, r);
					entry_start += 4;
				}
				Response::RegistersGet { registers }
			},
			0x62 => {
				// stopped
				let c = [buffer[1], buffer[0]];
				let mut pc = 0;
				for b in c.iter() {
					pc <<= 8;
					pc |= *b as u16;
				}
				Response::Stopped { pc }
			},
			0x63 => {
				// resumed
				let c = [buffer[1], buffer[0]];
				let mut pc = 0;
				for b in c.iter() {
					pc <<= 8;
					pc |= *b as u16;
				}
				Response::Resumed { pc }
			},
			0x71 => {
				// advance instructions
				Response::AdvanceInstructions // :TODO:
			},
			0x81 => {
				// ping
				Response::Ping
			},
			0x83 => {
				// registers available
				println!("Body for 0x83 - registers available");
				for b in buffer.iter() {
					print!("{:#02x} ", b);
				}
				println!("");

				/*
				byte 0-1: The count of the array items
				byte 2+: An array with items of structure:

				byte 0: Size of the item, excluding this byte
				byte 1: ID of the register
				byte 2: Size of the register in bits
				byte 3: Length of name
				byte 4+: Name
									*/

				let mut registers = HashMap::new();
				let count = (buffer[1] as u16) << 8 | (buffer[0] as u16);
				println!("Entry count {}", count);

				let mut entry_start = 2;
				for e in 0..count {
					let size = buffer[entry_start] as usize;
					let id = buffer[entry_start + 1];
					let r_size = buffer[entry_start + 2];
					let len = buffer[entry_start + 3] as usize;
					let mut name = Vec::new();
					for i in 0..=len {
						name.push(buffer[entry_start + 3 + i])
					}

					let name = match std::str::from_utf8(&name) {
						Ok(name) => name,
						Err(_e) => "[INVALID]",
					};

					println!(
						"{:#02} | {:#04x} {:#04x} {:#04x} -> {}",
						e, size, id, r_size, name
					);
					let r = (r_size, name.to_owned());
					registers.insert(id, r);
					entry_start += size + 1;
				}
				Response::RegistersAvailable { registers }
			},
			0xaa => {
				// exit
				Response::Exit
			},
			0xcc => {
				// reset
				Response::Reset
			},
			o => Response::Invalid,
		}
	}
}
