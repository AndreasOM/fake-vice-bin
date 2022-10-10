
use fake_vice_bin::FakeViceBin;


fn main() -> anyhow::Result<()> {
	let mut fvb = FakeViceBin::new( "127.0.0.1", 6502 );

	let short_delay = std::time::Duration::from_millis(5);
	let delay = std::time::Duration::from_millis(500);
	let long_delay = std::time::Duration::from_millis(5000);

	fvb.connect()?;
	fvb.send_registers_available( 0 )?;
	
	println!("Reset");
	std::thread::sleep(delay);
//	fvb.connect()?;
	fvb.update()?;
	fvb.send_reset()?;
//	fvb.disconnect()?;
	
	while fvb.is_reset_pending() {
		std::thread::sleep(delay);
		fvb.update()?;
	}
	
	println!("Exit");
	std::thread::sleep(delay);
//	fvb.connect()?;
	fvb.update()?;
	fvb.send_exit()?;
//	fvb.disconnect()?;
	
	
	println!("Load");
	std::thread::sleep(delay);
//	fvb.connect()?;
	fvb.send_load("main.prg", true)?;
	fvb.send_exit()?;
	//fvb.send_ping()?;
//	fvb.disconnect()?;

	std::thread::sleep(long_delay);

/*
	while fvb.is_load_pending() {
		std::thread::sleep(delay);
		fvb.update()?;
	}
*/	
//	fvb.connect()?;
//	loop {
		fvb.send_advance_instructions( 1000 )?;
		//fvb.send_ping()?;
		//fvb.send_exit()?;
		fvb.update()?;
		std::thread::sleep(short_delay);
//	}
	loop {
		fvb.update()?;
		//fvb.send_ping()?;
		fvb.send_exit()?;
		std::thread::sleep(delay);
	}
	Ok(())
}
