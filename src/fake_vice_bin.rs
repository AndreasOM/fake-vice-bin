use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::HeapRb;

use crate::Response;
use crate::ResponseHeader;

#[derive(Debug, Default)]
pub struct Register {
	name:  String,
	value: u16,
	size:  u8, // in bits
}

impl Register {
	pub fn set_name(&mut self, name: &str) {
		self.name = name.to_owned();
	}
	pub fn set_value(&mut self, value: u16) {
		self.value = value;
	}
	pub fn set_size(&mut self, size: u8) {
		self.size = size;
	}
	pub fn name(&self) -> &str {
		&self.name
	}
	pub fn value(&self) -> u16 {
		self.value
	}
	pub fn size(&self) -> u8 {
		self.size
	}
}

//#[derive(Debug)]
pub struct FakeViceBin {
	socket_addr:     SocketAddr,
	//stream:          Option<TcpStream>,
	resets_pending:  usize,
	load_pending:    bool,
	next_request_id: u32,
	running:         bool,
	program_counter: u16,
	registers:       HashMap<u8, Register>,

	response_rb_prod: Option<Producer<u8, Arc<HeapRb<u8>>>>,
	response_rb_cons: Option<Consumer<u8, Arc<HeapRb<u8>>>>,
	request_rb_prod:  Option<Producer<u8, Arc<HeapRb<u8>>>>,
	request_rb_cons:  Option<Consumer<u8, Arc<HeapRb<u8>>>>,
	connected:        bool,
}

impl FakeViceBin {
	pub fn new(host: &str, port: u16) -> Self {
		let ip: IpAddr = IpAddr::from_str(host).expect("...");
		Self {
			socket_addr:      (ip, port).into(),
			//stream:           None,
			//response_buffer: VecDeque::new(),
			resets_pending:   0,
			load_pending:     false,
			next_request_id:  0,
			running:          true,
			program_counter:  0,
			registers:        HashMap::default(),
			response_rb_prod: None, //prod,
			response_rb_cons: None, //cons,
			request_rb_prod:  None,
			request_rb_cons:  None,
			connected:        false,
		}
	}

	pub fn is_connected(&self) -> bool {
		self.connected
	}

	pub fn connect(&mut self) -> anyhow::Result<()> {
		if self.connected {
			anyhow::bail!("Already connected!");
		}
		match TcpStream::connect(self.socket_addr) {
			Ok(mut stream) => {
				stream.set_nonblocking(true)?;
				//stream.set_nodelay(true)?; // maybe not

				let rb = HeapRb::<u8>::new(64 * 1024); // this should be more than plenty, well, it's too large, but I have a plan
				let (prod, cons) = rb.split();
				self.response_rb_prod = Some(prod);
				self.response_rb_cons = Some(cons);

				let rb = HeapRb::<u8>::new(64 * 1024); // this should be more than plenty, well, it's too large, but I have a plan
				let (prod, cons) = rb.split();
				self.request_rb_prod = Some(prod);
				self.request_rb_cons = Some(cons);

				self.connected = true;

				let mut response_rb_prod = self.response_rb_prod.take();
				let mut request_rb_cons = self.request_rb_cons.take();

				let socket_addr = self.socket_addr.to_string();
				thread::spawn(move || -> anyhow::Result<()> {
					let delay = std::time::Duration::from_millis(5);
					loop {
						// receive
						let mut buf = [0; 1];
						loop {
							let _size = match stream.read(&mut buf) {
								Ok(size) => {
									//println!("Read {} bytes from stream", size);
									size
								},
								Err(ref e) => {
									match e.kind() {
										std::io::ErrorKind::WouldBlock => {
											//println!("No updates");
											break;
										},
										e => {
											anyhow::bail!(
												"Error reading from to {}: {}",
												&socket_addr,
												e
											);
										},
									}
								},
							};

							if let Some(response_rb_prod) = &mut response_rb_prod {
								while response_rb_prod.free_len() < buf.len() {
									// spin until there is space
									print!(".");
									let short_delay = std::time::Duration::from_millis(1);
									std::thread::sleep(short_delay);
								}
								let l = response_rb_prod.push_slice(&buf);
								/*
								for b in buf.iter() {
									match response_rb_prod.push(*b) {
										Ok(()) => {},
										Err(e) => {
											anyhow::bail!("Error storing response {}", e);
										},
									}
								}
								*/
							}
						}
						// println!("Read {} bytes in update", self.response_buffer.len() );

						// send
						//stream.write(buffer)?;
						if let Some(request_rb_cons) = &mut request_rb_cons {
							let len = request_rb_cons.len();
							if len > 0 {
								let mut buffer_vec = Vec::with_capacity(len);
								buffer_vec.resize(len, 0);
								let mut buffer = &mut buffer_vec[0..len];
								let l = request_rb_cons.pop_slice(&mut buffer);
								println!("Got {} bytes from ringbuffer for sending", l);
								stream.write(buffer)?;
							}
						};
						thread::sleep(delay);
					}
					Ok(())
				});
				Ok(())
			},
			Err(e) => {
				anyhow::bail!("Error connecting to {}: {}", &self.socket_addr, e);
			},
		}
	}
	pub fn disconnect(&mut self) -> anyhow::Result<()> {
		Ok(())

		// :TODO:
		/*
		if let Some(stream) = &mut self.stream.take() {
			stream.shutdown(std::net::Shutdown::Both)?;
			self.response_rb_prod = None;
			self.response_rb_cons = None;

			self.connected = false;
			Ok(())
		} else {
			Ok(())
		}
		*/
	}

	pub fn is_reset_pending(&self) -> bool {
		self.resets_pending > 0
	}
	pub fn is_load_pending(&self) -> bool {
		self.load_pending
	}

	fn generate_request_id(&mut self) -> [u8; 4] {
		let id = self.next_request_id;
		self.next_request_id += 1;
		[
			(id >> 0 & 0xff) as u8,
			(id >> 1 & 0xff) as u8,
			(id >> 2 & 0xff) as u8,
			(id >> 3 & 0xff) as u8,
		]
	}
	fn build_command(&mut self, command: u8, mut body: Vec<u8>) -> Vec<u8> {
		let l = body.len();
		let mut buffer = Vec::with_capacity(12 + l);
		buffer.push(0x02); // STX
		buffer.push(0x02); // version
				   // 2-5 body length
		buffer.push(l as u8); // 0x00; // :HACK:
		buffer.push(0x00);
		buffer.push(0x00);
		buffer.push(0x00);
		// 6-9 request id -> little endian!
		let rid = self.generate_request_id();
		buffer.push(rid[0]);
		buffer.push(rid[1]);
		buffer.push(rid[2]);
		buffer.push(rid[3]);
		// 10 command id
		buffer.push(command);
		// 11 command body
		buffer.append(&mut body);

		buffer
	}

	fn send_buffer(&mut self, buffer: &[u8]) -> anyhow::Result<()> {
		if let Some(request_rb_prod) = &mut self.request_rb_prod {
			for b in buffer.iter() {
				match request_rb_prod.push(*b) {
					Ok(()) => {},
					Err(e) => {
						anyhow::bail!("Error storing request {}", e);
					},
				}
			}
			Ok(())
		} else {
			anyhow::bail!("No buffer when trying to send");
		}
	}

	fn handle_response(&mut self) -> anyhow::Result<()> {
		if let Some(response_buffer_cons) = &mut self.response_rb_cons {
			// occupied_len
			if response_buffer_cons.len() >= 12 {
				let mut header_buffer = [0u8; 12];
				let l = response_buffer_cons.pop_slice(&mut header_buffer);
				println!("Got {} bytes from ringbuffer for header", l);
				if l != 12 {
					anyhow::bail!("Short header {} != 12", l);
				}
				let rh: ResponseHeader = (&header_buffer).into();
				if !rh.valid() {
					anyhow::bail!("Invalid response header!");
				}
				let body_len = rh.body_len() as usize;
				let mut body_vec = Vec::with_capacity(body_len);
				body_vec.resize(body_len, 0);
				let mut body_buffer = &mut body_vec[0..body_len]; //body_vec.as_slice();
				while body_len > response_buffer_cons.len() {
					print!(".");
					let short_delay = std::time::Duration::from_millis(1);
					std::thread::sleep(short_delay);
				}
				let l = response_buffer_cons.pop_slice(&mut body_buffer);
				println!("Got {} bytes from ringbuffer for body (Response Type: {:#04x}, Error Code: {:#04x})", l, rh.response_type(), rh.error_code());
				if l != body_len {
					anyhow::bail!("Short body {} != {}", l, body_len);
				}

				let r: Response = (&rh, &body_buffer[..]).into();

				match r {
					Response::RegistersGet { registers } => {
						for (k, v) in registers {
							let id = k;
							let size = v.0;
							let value = v.1;
							let r = self
								.registers
								.entry(id)
								.or_insert_with(|| Register::default());
							println!("{:#04x} {:#04x} {:#06x} | {}", size, id, value, r.name());
							r.set_value(value);
						}
					},
					Response::Stopped { pc } => {
						self.running = false;
						self.program_counter = pc;
						//println!("stopped PC {:#06x}", pc);
					},
					Response::Resumed { pc } => {
						self.running = true;
						self.program_counter = pc;
						//println!("resumed PC {:#06x}", pc);
					},
					/*
					0x71 => { // advance instructions
					},
					0x81 => { // ping
					},
					*/
					Response::RegistersAvailable { registers } => {
						for (k, v) in registers {
							let id = k;
							let r_size = v.0;
							let name = v.1;

							println!("{:#04x} {:#04x} -> {}", id, r_size, name);
							let r = self
								.registers
								.entry(id)
								.or_insert_with(|| Register::default());
							r.set_name(&name);
							r.set_size(r_size);
						}
					},
					Response::Exit => {},
					Response::Reset => {
						// reset
						println!("Handled reset");
						self.resets_pending -= 1;
					},
					_o => match rh.error_code() {
						0x80 => {
							println!("Invalid command length for {:#010x}", rh.request_id());
						},
						ec => {
							println!(
								"Unhandled response type {:#04x} (error code: {:#04x})",
								rh.response_type(),
								ec
							);
						},
					},
				}
			}
		}

		Ok(())
	}

	pub fn update(&mut self) -> anyhow::Result<()> {
		println!("{} {:?}", &self.resets_pending, self.load_pending);
		if self.connected {
			loop {
				let l = if let Some(response_buffer_cons) = &mut self.response_rb_cons {
					response_buffer_cons.len()
				} else {
					0
				};
				if l >= 12 {
					self.handle_response()?;
				} else {
					break;
				}
			}

			Ok(())
		} else {
			anyhow::bail!("Not connected to read update");
		}
	}

	pub fn send_ping(&mut self) -> anyhow::Result<()> {
		if self.connected {
			let mut buf = Vec::new();

			buf.push(0x02); // STX
			buf.push(0x02); // version
				// 2-5 body length
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			// 6-9 request id -> little endian!
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			// 10 command id
			buf.push(0x81); // 0x81 -> ping
				// 11 command body

			self.send_buffer(&buf)
		} else {
			anyhow::bail!("Not connected to send ping");
		}
	}

	pub fn send_exit(&mut self) -> anyhow::Result<()> {
		if self.connected {
			let mut buf = Vec::new();

			buf.push(0x02); // STX
			buf.push(0x02); // version
				// 2-5 body length
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			// 6-9 request id -> little endian!
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			buf.push(0x00);
			// 10 command id
			buf.push(0xaa); // 0xaa -> exit
				// 11 command body

			self.send_buffer(&buf)
		} else {
			anyhow::bail!("Not connected to send exit");
		}
	}

	pub fn send_reset(&mut self) -> anyhow::Result<()> {
		if self.connected {
			let mut body = Vec::new();
			body.push(0x01); // 0x01 -> hard reset

			let buf = self.build_command(0xcc, body);
			self.resets_pending += 1;
			self.send_buffer(&buf)
		} else {
			anyhow::bail!("Not connected to send reset");
		}
	}
	pub fn send_load(&mut self, filename: &str, autostart: bool) -> anyhow::Result<()> {
		if self.connected {
			let filename_bytes: &[u8] = filename.as_bytes();
			// :TODO: ascii cleanup/check
			let filename_len = filename_bytes.len();
			// :TODO: check length

			let mut body = Vec::new();
			if autostart {
				body.push(0x01); // autostart
			} else {
				body.push(0x00); // no autostart
			}
			// file index of disk image
			body.push(0x00);
			body.push(0x00);
			body.push(filename_len as u8);
			for b in filename_bytes.iter() {
				body.push(*b);
			}

			let buf = self.build_command(0xdd, body);
			self.send_buffer(&buf)
		} else {
			anyhow::bail!("Not connected to send load");
		}
	}
	pub fn send_registers_available(&mut self, memspace: u8) -> anyhow::Result<()> {
		println!("send_registers_available");
		if self.connected {
			let mut body = Vec::new();
			body.push(memspace);

			let buf = self.build_command(0x83, body);
			self.send_buffer(&buf)
		} else {
			anyhow::bail!("Not connected to send registers available");
		}
	}
	pub fn send_advance_instructions(&mut self, count: u16) -> anyhow::Result<()> {
		if self.connected {
			let mut body = Vec::new();
			body.push(0); // do not step over subroutines
			body.push(((count >> 0) & 0xff) as u8);
			body.push(((count >> 1) & 0xff) as u8);

			let buf = self.build_command(0x71, body);
			self.send_buffer(&buf)
		} else {
			anyhow::bail!("Not connected to send registers available");
		}
	}
}
