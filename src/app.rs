use std::{fs::File, time::Duration};
use std::env::*;
use std::io::{Read, Cursor};

use crossterm::event::{
	self, 
	Event
};
use ratatui::{
	layout::Alignment, prelude::Widget, style::{
		Color, Style, Stylize
	}, text::{
		Text, 
		Span
	}, widgets::{
		canvas::{
			Canvas,
			Line,
			Map,
			MapResolution,
			Rectangle,
		}, Block, Paragraph, Wrap
	}, DefaultTerminal, Frame 
};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

use color_eyre::Result;

#[derive(Debug, Clone)]
pub struct Program{
	name: String,
	content: String,
	finished: bool,
}

impl Program {
    pub fn new(name: String, content: String) -> Self {
        Self { name, content, finished: false }
    }
}

fn rainbow_wheel(mut wheel_pos: u8) -> Color {
	wheel_pos = 255 - wheel_pos;
	if wheel_pos < 85 {
		Color::Rgb(255 - wheel_pos * 3, 0, wheel_pos * 3)
	} else if wheel_pos < 170 {
		wheel_pos -= 85;
		Color::Rgb(0, wheel_pos * 3, 255 - wheel_pos * 3)
	} else {
		wheel_pos -= 170;
		Color::Rgb(wheel_pos * 3, 255 - wheel_pos * 3, 0)
	}
}

// #[derive(Debug)]
pub struct App{
	// Programs
	programs: Vec<Program>,
	current: usize,
	finished: bool,
	typed: String,
	// Color Effect
	wheel_pos: u8,
	rainbow: bool,
	typed_color: Color,
	// Sound Effect
	stream: OutputStream,
	sink: Sink,
}

impl App {
	pub fn new() -> Self{
		let mut args: Vec<String> = args().collect();
		args.remove(0);
		println!("args here :=> {:?}", args);
		
		let mut programs = vec![];
		
		for path in args.iter(){
			programs.push(Program::new(path.clone(), load_file(&path.as_str()) ));
		}
		
		println!("{}",programs.len());
		
		let stream = OutputStreamBuilder::open_default_stream()
			.expect("Could not open default audio stream");
		let mixer = stream.mixer();
		let sink = Sink::connect_new(&mixer);
		
		let sound_data = include_bytes!("assets/blipSelect.wav");
		let cursor = Cursor::new(sound_data);
		let source = Decoder::new(cursor).unwrap();
		
		// Append to sink to play without blocking
		sink.append(source);
		
		Self{
			programs,
			current: 0,
			finished: false,
			typed: String::new(),
			wheel_pos: 0,
			rainbow: true,
			typed_color: Color::Yellow,
			stream: stream,
			sink,
		}
	}
	pub fn get_current(&self) -> Option<Program>{
		if self.programs.is_empty(){
			return None
		}
		Some(self.programs[self.current].clone())
	}
	pub fn check_char_guess(&mut self, c: char){
		let mut current_program = self.get_current().expect("no programs lol");
		
		let target = current_program.content.chars().nth(self.typed.len());
		
		match target{
			Some(nc) => {
				if nc == c{
					self.typed.push(c);
					self.play_correct_sound();
				} else{
					self.play_error_sound();
				}
			},
			None => self.finished = true,
		}
	}
	pub fn check_enter_guess(&mut self) {
		let current_program = self.get_current().expect("no programs lol");
		
		// Get the character at the current cursor position
		let target = current_program.content.chars().nth(self.typed.len());
		
		match target {
			Some('\n') => {
				// It's a match! User pressed Enter and the text expects a newline.
				self.typed.push('\n');
				self.play_correct_sound();
			},
			Some(_) => {
				self.play_error_sound();
			},
			None => {
				// We reached the end of the content
				self.finished = true;
			}
		}
	}
	pub fn check_tab_guess(&mut self) {
		let current_program = self.get_current().expect("no programs lol");
		
		// Get the character at the current cursor position
		let target = current_program.content.chars().nth(self.typed.len());
		
		match target {
			Some('\t') => {
				// It's a match! User pressed Enter and the text expects a newline.
				self.typed.push('\t');
				self.play_correct_sound();
			},
			Some(_) => {
				self.play_error_sound();
			},
			None => {
				// We reached the end of the content
				self.finished = true;
			}
		}
	}
	pub fn play_error_sound(&mut self) {
		let sound_data = include_bytes!("assets/hitHurt.wav");
		let cursor = Cursor::new(sound_data);
		let source = Decoder::new(cursor).unwrap();

		self.sink.append(source);
		self.sink.play();
		if !self.typed.is_empty(){
			self.typed.remove(self.typed.len().saturating_sub(1));
		}
		// Hardcore mode 
		self.typed = String::new() 
	}
	pub fn play_correct_sound(&self) {
		let sound_data = include_bytes!("assets/click.wav");
		let cursor = Cursor::new(sound_data);
		let source = Decoder::new(cursor).unwrap();
		
		self.sink.append(source);
		self.sink.play();
	}
	pub fn get_cursor_position(&self, tab_char: &str) -> (u16, u16) {
		let mut x: u16 = 0;
		let mut y: u16 = 0;
		
		for c in self.typed.chars() {
			if c == '\n' {
				x = 0;
				y += 1;
			} else if c == '\t' {
				x += tab_char.len() as u16;
			} else {
				x += 1;
			}
		}
		(y, 0)
	}
}

fn render(frame: &mut Frame, app: &mut App) {
	let current_program = app.get_current().expect("no programs lol");
	
	let content = current_program.content.clone();
	let thing: Vec<char> = content.chars()
	.skip(app.typed.len())
	.collect();
	
	let usable_content = {
		let mut tmp = String::new();
		for c in thing.iter(){
			tmp.push(*c);
		}
		tmp
	};
	
	let full_content = format!("{}{}", app.typed, usable_content);
	let typed_len = app.typed.len();
	let mut current_pos = 0;
	let mut lines = Vec::new();
	let (tab_char, space_char) = include!("chars.rs");
	let typed_color = app.typed_color;
	
	// Iterate through each line of the FULL text
	for raw_line in full_content.split('\n') {
		let mut spans = Vec::new();
		// Use .chars().count() to stay consistent with typed_len logic
		let line_char_count = raw_line.chars().count();
		
		// Check where this line stands relative to the 'typed' boundary
		if current_pos + line_char_count <= typed_len {
			// ENTIRE LINE is typed (Green)
			let display = raw_line.replace(' ', space_char).replace('\t', tab_char);
			spans.push(Span::styled(display, Style::default().fg(typed_color)));
		} else if current_pos >= typed_len {
			// ENTIRE LINE is remaining (White)
			let display = raw_line.replace(' ', space_char).replace('\t', tab_char);
			spans.push(Span::styled(display, Style::default().fg(Color::White)));
		} else {
			// LINE IS SPLIT: Part Green, Part White
			let split_idx = typed_len - current_pos;
			
			// Split using characters to safely handle the transition
			let done_raw: String = raw_line.chars().take(split_idx).collect();
			let rest_raw: String = raw_line.chars().skip(split_idx).collect();
			
			// Expand tabs for display
			let display_done = done_raw.replace(' ', space_char).replace('\t', tab_char);
			let display_rest = rest_raw.replace(' ', space_char).replace('\t', tab_char);
			
			spans.push(Span::styled(display_done, Style::default().fg(typed_color)));
			spans.push(Span::styled(display_rest, Style::default().fg(Color::White)));
		}
		
		lines.push(ratatui::text::Line::from(spans));
		
		// Add 1 to account for the '\n' character we split on
		current_pos += line_char_count + 1;
	}
	
	// Create the final Text object
	let final_ui_text = ratatui::text::Text::from(lines);
	
	let target = current_program.content.chars().nth(app.typed.len());
	
	let text = Paragraph::new(final_ui_text)
		.scroll(app.get_cursor_position(tab_char))
		.block(Block::bordered().title(format!("{} -- current char: [ {:?} ]", current_program.name, target)));
	frame.render_widget(text, frame.area());
}

pub fn load_file(path: &str) -> String{
	let file = File::open(path);
	match file {
		Ok(mut f) => {
			let mut contents = String::new();
			match f.read_to_string(&mut contents){
				Ok(_) => (),
				Err(e) => panic!("Couldnt read file: {}", e),
			}
			contents
		},
		Err(e) => panic!("Couldnt open file: {}", e),
	}
}

pub fn run(mut terminal: DefaultTerminal, app: &mut App) -> Result<()>{
	loop {
		if app.rainbow{
			app.wheel_pos = app.wheel_pos.wrapping_add(10);
			app.typed_color = rainbow_wheel(app.wheel_pos);
		} else{
			app.typed_color = Color::Yellow;
		}
		
		if app.finished{
			if app.programs.len() == 1{
				println!("{}",app.programs.len());
				break Ok(());
			} else {
				app.programs.remove(0);
				app.finished = false;
				app.typed = String::new();
			}
		}
		
		if event::poll(Duration::from_millis(16))?{
			if let Event::Key(key) = event::read()? {
				match key.code {
					event::KeyCode::Esc => {
						break Ok(());
					},
					event::KeyCode::Enter => {
						app.check_enter_guess();
					}
					event::KeyCode::Tab => {
						app.check_tab_guess();
					}
					event::KeyCode::Char(c) => {
						app.check_char_guess(c);
					}
					_ => (),
				}
			}
		}
		
		terminal.draw(|f| 
		render(f, app)
		)?;
	}
}

