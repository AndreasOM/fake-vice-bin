use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};

use fake_vice_bin::FakeViceBin;

#[derive(Debug, Default)]
enum Command {
	#[default]
	None,
	Connect,
	SendRegistersAvailable {
		mem: u8,
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

	fn add_send_registers_available(&mut self, mem: u8) {
		let c = Command::SendRegistersAvailable { mem };
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
			} else if let Some(jump) = cmd.strip_prefix("connect(") {
				if let Some(_) = jump.strip_suffix(")") {
					self.add_connect();
				} else {
					anyhow::bail!("Missing closing ) on connect in line {}", line_no);
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
			} else {
				//anyhow::bail!("Unkown command >{}< in line {}", &s, line_no );
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

			let c = &self.commands[pc];
			match c {
				Command::Connect => {
					fvb.connect()?;
					// :TODO: block
				},
				Command::SendRegistersAvailable { mem } => {
					fvb.send_registers_available(*mem)?;
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
