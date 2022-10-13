use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::str::FromStr;

use fake_vice_bin::FakeViceBin;

#[derive(Debug, Default)]
enum Command {
	#[default]
	None,
	BlockDuringReset,
	Connect,
	Update,
	SendLoad {
		filename:  String,
		autostart: bool,
	},
	SendRegistersAvailable {
		mem: u8,
	},
	SendAdvanceInstructions {
		count: u16,
	},
	SendReset,
	SendExit,
	Sleep {
		seconds: f32,
	},
	Jump {
		target: String,
	},
	Label {
		name: String,
	},
}

#[derive(Debug, Default)]
pub struct Script {
	commands: Vec<Command>,
	labels:   HashMap<String, usize>,
}

impl Script {
	pub fn new() -> Self {
		Default::default()
	}

	fn add_connect(&mut self) {
		let c = Command::Connect;
		self.commands.push(c);
	}
	fn add_block_during_reset(&mut self) {
		let c = Command::BlockDuringReset;
		self.commands.push(c);
	}

	fn add_update(&mut self) {
		let c = Command::Update;
		self.commands.push(c);
	}

	fn add_send_reset(&mut self) {
		let c = Command::SendReset;
		self.commands.push(c);
	}

	fn add_send_exit(&mut self) {
		let c = Command::SendExit;
		self.commands.push(c);
	}

	fn add_send_load(&mut self, filename: &str, autostart: bool) {
		let c = Command::SendLoad {
			filename: filename.to_owned(),
			autostart,
		};
		self.commands.push(c);
	}

	fn add_send_registers_available(&mut self, mem: u8) {
		let c = Command::SendRegistersAvailable { mem };
		self.commands.push(c);
	}
	fn add_send_advance_instructions(&mut self, count: u16) {
		let c = Command::SendAdvanceInstructions { count };
		self.commands.push(c);
	}
	fn add_sleep(&mut self, seconds: f32) {
		let c = Command::Sleep { seconds };
		self.commands.push(c);
	}

	fn add_jump(&mut self, target: &str) {
		let c = Command::Jump {
			target: target.to_owned(),
		};
		self.commands.push(c);
	}

	fn add_label(&mut self, label: &str) {
		let c = Command::Label {
			name: label.to_owned(),
		};
		self.labels.insert(label.to_owned(), self.commands.len());
		self.commands.push(c);
	}
	fn add_from_str(&mut self, s: &str, line_no: usize) -> anyhow::Result<()> {
		// :TODO: some regexes might be better, or one of the parsing packages
		if let Some((label, _)) = s.split_once(":") {
			println!("Label >{}<", &label);
			self.add_label(&label);
		} else if let Some(_an_if) = s.strip_prefix("if") {
			println!(">if< on line {} not handled yet", line_no);
		} else if let Some(_an_if) = s.strip_prefix("{") {
			println!(">{{< on line {} not handled yet", line_no);
		} else if let Some(_an_if) = s.strip_prefix("}") {
			println!(">}}< on line {} not handled yet", line_no);
		} else if let Some(cmd) = s.strip_suffix(";") {
			if let Some(jump) = cmd.strip_prefix("jump(") {
				if let Some(jump) = jump.strip_suffix(")") {
					self.add_jump(&jump);
				} else {
					anyhow::bail!("Missing closing ) on jump in line {}", line_no);
				}
			} else if let Some(sleep) = cmd.strip_prefix("sleep(") {
				if let Some(sleep) = sleep.strip_suffix(")") {
					let s = f32::from_str(sleep).expect("NaN");
					self.add_sleep(s);
				} else {
					anyhow::bail!("Missing closing ) on sleep in line {}", line_no);
				}
			} else if let Some(connect) = cmd.strip_prefix("connect(") {
				if let Some(_) = connect.strip_suffix(")") {
					self.add_connect();
				} else {
					anyhow::bail!("Missing closing ) on connect in line {}", line_no);
				}
			} else if let Some(load) = cmd.strip_prefix("send_load(") {
				if let Some(l) = load.strip_suffix(")") {
					let params = l.split(",").collect::<Vec<&str>>();
					if params.len() == 2 {
						let filename = params[0].trim();
						let filename = filename
							.strip_prefix("\"")
							.expect("Missing opening \" for filename");
						let filename = filename
							.strip_suffix("\"")
							.expect("Missing closing \" for filename");
						let autostart = params[1].trim();
						let autostart = autostart == "true";
						self.add_send_load(filename, autostart);
					} else {
						anyhow::bail!(
							"Wrong number of parameters for send_load in line {}",
							line_no
						);
					}
				} else {
					anyhow::bail!("Missing closing ) on send_load in line {}", line_no);
				}
			} else if let Some(block_during_reset) = cmd.strip_prefix("block_during_reset(") {
				if let Some(_) = block_during_reset.strip_suffix(")") {
					self.add_block_during_reset();
				} else {
					anyhow::bail!(
						"Missing closing ) on block_during_reset in line {}",
						line_no
					);
				}
			} else if let Some(update) = cmd.strip_prefix("update(") {
				if let Some(_) = update.strip_suffix(")") {
					self.add_update();
				} else {
					anyhow::bail!("Missing closing ) on update in line {}", line_no);
				}
			} else if let Some(send_reset) = cmd.strip_prefix("send_reset(") {
				if let Some(_) = send_reset.strip_suffix(")") {
					self.add_send_reset();
				} else {
					anyhow::bail!("Missing closing ) on send_reset in line {}", line_no);
				}
			} else if let Some(send_exit) = cmd.strip_prefix("send_exit(") {
				if let Some(_) = send_exit.strip_suffix(")") {
					self.add_send_exit();
				} else {
					anyhow::bail!("Missing closing ) on send_exit in line {}", line_no);
				}
			} else if let Some(r) = cmd.strip_prefix("send_registers_available(") {
				if let Some(m) = r.strip_suffix(")") {
					let m = u8::from_str_radix(m, 10).expect("NaN");
					self.add_send_registers_available(m);
				} else {
					anyhow::bail!(
						"Missing closing ) on send_registers_available in line {}",
						line_no
					);
				}
			} else if let Some(i) = cmd.strip_prefix("send_advance_instructions(") {
				if let Some(c) = i.strip_suffix(")") {
					let c = u16::from_str_radix(c, 10).expect("NaN");
					self.add_send_advance_instructions(c);
				} else {
					anyhow::bail!(
						"Missing closing ) on send_advance_instructions in line {}",
						line_no
					);
				}
			} else {
				anyhow::bail!("Unkown command >{}< in line {}", &s, line_no);
			}
		} else {
			anyhow::bail!("Missing line end ; on command >{}< in line {}", &s, line_no);
		}
		Ok(())
	}
	pub fn load(&mut self, filename: &str) -> anyhow::Result<()> {
		let file = File::open(filename)?;
		let lines = io::BufReader::new(file).lines();
		for (line_no, line) in lines.enumerate() {
			if let Ok(line) = line {
				let line = line.split_once("//").unwrap_or((&line, "")).0;
				let line = line.trim();
				if line.is_empty() {
					continue;
				}
				println!("{:?}", &line);
				self.add_from_str(line, line_no)?;
			}
		}
		Ok(())
	}

	pub fn run(&mut self) -> anyhow::Result<()> {
		let mut fvb = FakeViceBin::new("127.0.0.1", 6502);
		let mut pc = 0;
		loop {
			if pc >= self.commands.len() {
				break;
			}

			//if fvb.is_connected() {
			//	fvb.update()?;
			//}
			let c = &self.commands[pc];
			println!("{:?}", &c);
			match c {
				Command::Connect => {
					fvb.connect()?;
					// :TODO: block
				},
				Command::Update => {
					fvb.update()?;
				},
				Command::BlockDuringReset => {
					let delay = std::time::Duration::from_millis(5);
					while fvb.is_reset_pending() {
						std::thread::sleep(delay);
						fvb.update()?; // :TODO: can be removed once update are applied in separate thread
					}
				},
				Command::SendLoad {
					filename,
					autostart,
				} => {
					fvb.send_load(filename, *autostart)?;
				},
				Command::SendRegistersAvailable { mem } => {
					fvb.send_registers_available(*mem)?;
				},
				Command::SendAdvanceInstructions { count } => {
					fvb.send_advance_instructions(*count)?;
				},
				Command::SendReset => {
					fvb.send_reset()?;
				},
				Command::SendExit => {
					fvb.send_exit()?;
				},
				Command::Sleep { seconds } => {
					let delay = std::time::Duration::from_millis((*seconds * 1000.0) as u64);
					std::thread::sleep(delay);
				},
				Command::Jump { target } => {
					if let Some(t) = self.labels.get(target) {
						pc = *t;
						continue;
					} else {
						anyhow::bail!("Label {} not found", target);
					}
				},
				Command::Label { name: _ } => {},
				Command::None => {},
			}
			pc += 1;
		}
		Ok(())
	}
}
