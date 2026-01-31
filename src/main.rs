use color_eyre::Result;

mod app;

use crate::app::*;

fn main() -> Result<()> {
	let mut app = App::new();
	color_eyre::install()?;
	let terminal = ratatui::init();
	let result = run(terminal, &mut app);
	ratatui::restore();
	result
}

