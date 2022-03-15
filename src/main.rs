mod engine;
mod machine;

use crossterm::event::Event;
use crossterm::style::{Print, Stylize};
use crossterm::{cursor, event, execute, queue, terminal};
use directories::ProjectDirs;
use engine::Action;
use enigo::{Enigo, Key, KeyboardControllable};
use std::io::{stdout, Write};
use std::{collections::VecDeque, fs, sync::mpsc};
use toml::Value;

pub enum Ui {
    Stroke(u32, u128, i32),
    Machine(String),
    DictionaryLoaded,
    Resize(u16, u16),
}

const DISPLAY_LEN: u16 = 25;

fn main() -> Result<(), std::io::Error> {
    let (tx, rx) = mpsc::channel();
    let tx1 = tx.clone(); // otherwise the main thread will end after panic
    std::thread::spawn(move || {
        steno_loop(tx);
    });
    std::thread::spawn(move || {
        event_loop(tx1).unwrap();
    });

    let mut stdout = stdout();
    let mut dim = terminal::size()?;
    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

    let mut display_buffer = VecDeque::new();
    let mut times_buffer = VecDeque::new();
    let mut efficiency_buffer = VecDeque::new();
    let mut dicts = 0;
    let mut machine_status = String::new();

    draw_dict_status(&mut stdout, dim, dicts)?;
    draw_machine_status(&mut stdout, dim, None)?;
    draw_stroke_display(&mut stdout, dim, &display_buffer, 0, 0.0, 0.0)?;
    stdout.flush()?;

    while let Ok(msg) = rx.recv() {
        match msg {
            Ui::Stroke(s, d, n) => {
                display_buffer.push_back(s);
                if d < 2500 {
                    times_buffer.push_back(d);
                }
                efficiency_buffer.push_back(n);
                if display_buffer.len() > DISPLAY_LEN.into() {
                    display_buffer.pop_front();
                    efficiency_buffer.pop_front();
                }
                if times_buffer.len() > DISPLAY_LEN.into() {
                    times_buffer.pop_front();
                }
                let avg =
                    1000.0 / (times_buffer.iter().sum::<u128>() as f64 / times_buffer.len() as f64);
                let efficiency =
                    efficiency_buffer.iter().sum::<i32>() as f64 / times_buffer.len() as f64;
                draw_stroke_display(&mut stdout, dim, &display_buffer, d, avg, efficiency)?;
            }
            Ui::Machine(s) => {
                machine_status.clear();
                machine_status.push_str(&s);
                if s.len() > 0 {
                    draw_machine_status(&mut stdout, dim, Some(s))?
                } else {
                    draw_machine_status(&mut stdout, dim, None)?
                }
            }
            Ui::DictionaryLoaded => {
                dicts += 1;
                draw_dict_status(&mut stdout, dim, dicts)?;
            }
            Ui::Resize(w, h) => {
                dim = (w, h);
                execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
                draw_dict_status(&mut stdout, dim, dicts)?;
                draw_machine_status(&mut stdout, dim, Some(machine_status.clone()))?;
                draw_stroke_display(&mut stdout, dim, &display_buffer, 0, 0.0, 0.0)?;
            }
        }
        stdout.flush()?;
    }
    Ok(())
}

fn event_loop(tx: mpsc::Sender<Ui>) -> crossterm::Result<()> {
    loop {
        match event::read()? {
            Event::Resize(w, h) => tx.send(Ui::Resize(w, h)).unwrap(),
            _ => {}
        }
    }
}

fn steno_loop(tx: mpsc::Sender<Ui>) {
    let proj_dirs = ProjectDirs::from("", "", "moreover").unwrap();
    let file = proj_dirs.config_dir().join("moreover.toml");
    let config = fs::read_to_string(file)
        .expect("Could not read config file")
        .parse::<Value>()
        .unwrap();

    let mut engine = engine::Engine::new();
    for dict in config["dictionaries"].as_array().unwrap() {
        tx.send(Ui::DictionaryLoaded).unwrap();
        engine.add_dict(dict.as_str().unwrap());
    }
    let mut machine = machine::Machine::new(
        config["machine"].as_str().unwrap().to_string(),
        config["keymap"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().chars().next().unwrap_or_default())
            .collect::<Vec<char>>()
            .try_into()
            .unwrap(),
    );
    tx.send(Ui::Machine(config["machine"].as_str().unwrap().to_string()))
        .unwrap();
    let mut enigo = Enigo::new();

    let mut time_start;

    loop {
        time_start = std::time::Instant::now();
        let stroke = machine.read(tx.clone()).expect("Unable to read stroke");
        if stroke == 0 {
            continue;
        }

        let (del, add) = engine.process_stroke(stroke);

        for a in &del {
            match a {
                Action::Text(_) => enigo.key_click(Key::Backspace),
                _ => {}
            }
        }
        for a in &add {
            match a {
                Action::Text(s) => match s.as_str() {
                    // key_sequence doesn't seem to work for newline, i have to do this stupid
                    "\n" => enigo.key_click(Key::Return),
                    _ => enigo.key_sequence(s),
                },
                Action::KeyClick(k) => enigo.key_click(*k),
                Action::KeyUp(k) => enigo.key_up(*k),
                Action::KeyDown(k) => enigo.key_down(*k),
            }
        }

        tx.send(Ui::Stroke(
            stroke,
            time_start.elapsed().as_millis(),
            add.len() as i32 - del.len() as i32,
        ))
        .unwrap();
    }
}

fn draw_stroke_display(
    stdout: &mut std::io::Stdout,
    dim: (u16, u16),
    s: &VecDeque<u32>,
    d: u128,
    a: f64,
    e: f64,
) -> Result<(), std::io::Error> {
    let w = engine::STENO_ORDER.len() + 2;
    let x = (dim.0 - w as u16 - 2) / 2;
    let y = (dim.1 - DISPLAY_LEN) / 2 - 1;
    queue!(
        stdout,
        cursor::MoveTo(x, y),
        Print(format!("┌{}┐", "─".repeat(w)).dark_grey()),
    )?;
    for i in 0..=DISPLAY_LEN {
        queue!(
            stdout,
            cursor::MoveTo(x, y + DISPLAY_LEN + 1 - i as u16),
            Print("│ ".dark_grey()),
            Print(
                engine::id_to_steno(if let Some(x) = s.get(s.len() - i as usize) {
                    *x
                } else {
                    0
                })
                .black()
            ),
            Print(" │".dark_grey()),
        )?;
    }
    queue!(
        stdout,
        cursor::MoveTo(x, y + DISPLAY_LEN + 1),
        Print(format!("└{}┘", "─".repeat(w)).dark_grey()),
    )?;
    let s = format!("SPS: {:.2}    Last: {}ms", a, d);
    let s2 = format!("CPS: {:.2}", e);
    let s3 = format!("(last {})", DISPLAY_LEN);
    queue!(
        stdout,
        cursor::MoveTo((dim.0 - s.len() as u16) / 2, (dim.1 + DISPLAY_LEN) / 2 + 2),
        terminal::Clear(terminal::ClearType::CurrentLine),
        Print(s),
        cursor::MoveTo((dim.0 - s2.len() as u16) / 2, (dim.1 + DISPLAY_LEN) / 2 + 3),
        terminal::Clear(terminal::ClearType::CurrentLine),
        Print(s2),
        cursor::MoveTo((dim.0 - s3.len() as u16) / 2, (dim.1 + DISPLAY_LEN) / 2 + 4),
        terminal::Clear(terminal::ClearType::CurrentLine),
        Print(s3.dark_grey()),
    )?;
    Ok(())
}

fn draw_machine_status(
    stdout: &mut std::io::Stdout,
    dim: (u16, u16),
    path: Option<String>,
) -> Result<(), std::io::Error> {
    let s = path.unwrap_or(String::from("disconnected"));
    queue!(
        stdout,
        cursor::MoveTo((dim.0 - s.len() as u16) / 2, (dim.1 - DISPLAY_LEN) / 2 - 4),
        terminal::Clear(terminal::ClearType::CurrentLine),
        Print(if s.as_str() != "disconnected" {
            s.green()
        } else {
            s.red()
        })
    )?;
    Ok(())
}
fn draw_dict_status(
    stdout: &mut std::io::Stdout,
    dim: (u16, u16),
    n: usize,
) -> Result<(), std::io::Error> {
    let s = format!("{} dictionaries loaded", n);
    queue!(
        stdout,
        cursor::MoveTo((dim.0 - s.len() as u16) / 2, (dim.1 - DISPLAY_LEN) / 2 - 3),
        terminal::Clear(terminal::ClearType::CurrentLine),
        Print(s),
    )?;
    Ok(())
}
