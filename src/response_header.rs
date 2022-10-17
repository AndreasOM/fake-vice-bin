#[derive(Debug, Default)]
pub struct ResponseHeader {
	valid:         bool,
	stx:           u8,
	version:       u8,
	body_len:      u32,
	response_type: u8,
	error_code:    u8,
	request_id:    u32,
}

impl ResponseHeader {
	pub fn valid(&self) -> bool {
		self.valid
	}
	pub fn stx(&self) -> u8 {
		self.stx
	}
	pub fn version(&self) -> u8 {
		self.version
	}
	pub fn body_len(&self) -> u32 {
		self.body_len
	}
	pub fn response_type(&self) -> u8 {
		self.response_type
	}
	pub fn error_code(&self) -> u8 {
		self.error_code
	}
	pub fn request_id(&self) -> u32 {
		self.request_id
	}
}
impl From<&[u8; 12]> for ResponseHeader {
	fn from(buffer: &[u8; 12]) -> Self {
		let mut rh = ResponseHeader::default();
		rh.valid = true;
		rh.stx = buffer[0];
		rh.version = buffer[1];

		rh.body_len = (buffer[2] as u32) << 0
			| (buffer[3] as u32) << 8
			| (buffer[4] as u32) << 16
			| (buffer[5] as u32) << 24;

		rh.response_type = buffer[6];
		rh.error_code = buffer[7];

		rh.request_id = (buffer[8] as u32) << 0
			| (buffer[9] as u32) << 8
			| (buffer[10] as u32) << 16
			| (buffer[11] as u32) << 24;

		if rh.stx != 0x02 {
			rh.valid = false;
		}
		if rh.version != 0x02 {
			rh.valid = false;
		}
		rh
	}
}
