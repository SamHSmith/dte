use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use std::io::{stdin, stdout, Write};

enum FileMode {
    Edit,
    Open,
    SaveAsPrompt,
}

fn line_wrap_count(text: &str, width: usize) -> usize {
    let mut len = text.len();
    let mut t = 0;
    while len > width {
        len -= width;
        t += 1;
    }
    return t;
}

fn main() {
    {
        let mut args: Vec<String> = std::env::args().collect();

        let stdin = termion::async_stdin();
        let mut stdin_keys = stdin.keys();
        let mut stdout = stdout().into_raw_mode().unwrap();

        let mut file_path = std::path::PathBuf::from(".".to_owned());
        file_path = file_path.canonicalize().unwrap();
        let mut should_load_file = file_path.to_str().unwrap().len() > 1 && file_path.exists();
        if args.len() > 1 {
            file_path.push(args.remove(1));
        } else {
            should_load_file = false;
        }

        let mut render_buffer: String = String::new();
        let mut width: usize;
        let mut height: usize;
        {
            let (w, h) = termion::terminal_size().unwrap();
            width = w as usize;
            height = h as usize;
        }
        let mut cursor_line: isize = 0;
        let mut cursor_column: isize = 0;
        let mut window_padding = 6;
        let mut window_start = 0;
        let mut running = true;

        let mut current_indentation = 0;

        let mut insert_mode = false;
        let mut file_mode = FileMode::Edit;

        let mut buffer = Vec::new();
        for y in 0..1 {
            buffer.push(String::with_capacity(width));
        }
        let mut bottom_bar_buffer = String::with_capacity(width);

        let mut last_cursor_flip = std::time::Instant::now();
        let mut cursor_on = false;
        const cursor_cycle_duration: u128 = 1000;

        while running {
            {
                let (w, h) = termion::terminal_size().unwrap();
                width = w as usize;
                height = h as usize;
            }

            if should_load_file {
                use std::fs::File;
                use std::io::BufRead;
                let f = File::open(&file_path).unwrap();
                let mut f = std::io::BufReader::new(f);
                let mut index = 0;
                loop {
                    if index < buffer.len() {
                        buffer[index].clear();
                    } else {
                        buffer.push(String::with_capacity(width));
                    }
                    match std::io::BufRead::read_line(&mut f, &mut buffer[index]) {
                        Ok(0) => break,
                        Ok(_) => (),
                        _ => panic!(),
                    }
                    let newlen = buffer[index].len().max(1) - 1;
                    buffer[index].truncate(newlen);
                    index += 1;
                }
                should_load_file = false;
            }

            loop {
                match stdin_keys.next() {
                    Some(c) => {
                        match file_mode {
                            FileMode::Open => match c.unwrap() {
                                Key::Char('\n') => {
                                    let path = std::path::Path::new(&bottom_bar_buffer);
                                    if path.parent().is_some()
                                        && path.parent().unwrap().exists()
                                        && !path.is_dir()
                                    {
                                        if file_path.exists() && !path.exists() {
                                            buffer.truncate(1);
                                            buffer[0].clear();
                                        }
                                        file_mode = FileMode::Edit;
                                        file_path = path.to_owned();
                                        should_load_file = file_path.exists();
                                        cursor_line = 0;
                                        cursor_column = 0;
                                    }
                                }
                                Key::Char(c) => {
                                    bottom_bar_buffer.push(c);
                                }
                                Key::Backspace => {
                                    bottom_bar_buffer.pop();
                                }
                                Key::Esc => {
                                    file_mode = FileMode::Edit;
                                }
                                _ => (),
                            },
                            FileMode::SaveAsPrompt => match c.unwrap() {
                                Key::Char('\n') => {
                                    let path = std::path::Path::new(&bottom_bar_buffer);
                                    if !path.is_dir() {
                                        use std::fs::OpenOptions;
                                        use std::io::{BufWriter, Write};
                                        let mut f = OpenOptions::new()
                                            .read(true)
                                            .write(true)
                                            .create(true)
                                            .open(path)
                                            .unwrap();

                                        for b in buffer.iter() {
                                            f.write_all(b.as_bytes()).unwrap();
                                            f.write_all(b"\n");
                                        }
                                    }
                                    file_mode = FileMode::Edit;
                                    file_path.clear();
                                    file_path.push("./");
                                    file_path = file_path.canonicalize().unwrap();
                                    file_path.push(std::path::Path::new(&bottom_bar_buffer));
                                    should_load_file = file_path.exists();
                                    cursor_line = 0;
                                    cursor_column = 0;
                                }
                                Key::Char(c) => {
                                    bottom_bar_buffer.push(c);
                                }
                                Key::Backspace => {
                                    bottom_bar_buffer.pop();
                                }
                                Key::Esc => {
                                    file_mode = FileMode::Edit;
                                }
                                _ => (),
                            },
                            FileMode::Edit => {
                                if insert_mode {
                                    match c.unwrap() {
                                        Key::Char('\n') => {
                                            let newstr = buffer[cursor_line as usize]
                                                .split_off(cursor_column as usize);
                                            buffer.insert(cursor_line as usize + 1, newstr);
                                            cursor_column = current_indentation;
                                            cursor_line += 1;
                                        }
                                        Key::Char('\t') => {
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                            for x in 0..4 {
                                                buffer[cursor_line as usize]
                                                    .insert(cursor_column as usize, ' ');
                                                cursor_column += 1;
                                            }
                                        }
                                        Key::Char(c) => {
                                            buffer[cursor_line as usize]
                                                .insert(cursor_column as usize, c);
                                            cursor_column += 1;
                                        }
                                        Key::Backspace => {
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                            if cursor_column > 0 {
                                                cursor_column -= 1;
                                                buffer[cursor_line as usize]
                                                    .remove(cursor_column as usize);
                                            } else {
                                                let st = buffer.remove(cursor_line as usize);
                                                let length = buffer[cursor_line as usize - 1].len();
                                                buffer[cursor_line as usize - 1]
                                                    .insert_str(length, &st);
                                                cursor_line -= 1;
                                                cursor_column = length as isize;
                                            }
                                        }
                                        Key::Esc => insert_mode = false,
                                        _ => (),
                                    }
                                } else {
                                    match c.unwrap() {
                                        // Exit.
                                        Key::Char('q') => running = false,
                                        Key::Char('f') => {
                                            file_mode = FileMode::Open;
                                            bottom_bar_buffer.clear();
                                            bottom_bar_buffer.insert_str(
                                                0,
                                                if !file_path.is_dir() {
                                                    file_path
                                                        .as_path()
                                                        .parent()
                                                        .unwrap()
                                                        .to_str()
                                                        .unwrap()
                                                } else {
                                                    file_path.as_path().to_str().unwrap()
                                                },
                                            );
                                            bottom_bar_buffer.push('/');
                                        }
                                        Key::Char('w') => {
                                            file_mode = FileMode::SaveAsPrompt;
                                        }
                                        Key::Char('t') => {
                                            cursor_column += 1;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('h') => {
                                            cursor_column -= 1;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('e') => {
                                            cursor_line += 1;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('u') => {
                                            cursor_line -= 1;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('E') => {
                                            cursor_line += 20;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('U') => {
                                            cursor_line -= 20;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('i') => {
                                            while cursor_line as usize >= buffer.len() {
                                                buffer.push(String::with_capacity(width));
                                            }
                                            insert_mode = true;
                                            while cursor_column as usize
                                                > buffer[cursor_line as usize].len()
                                            {
                                                buffer[cursor_line as usize].push(' ');
                                            }
                                        }
                                        Key::Char('d') => {
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                            if (cursor_column as usize)
                                                < buffer[cursor_line as usize].len()
                                            {
                                                buffer[cursor_line as usize]
                                                    .remove(cursor_column as usize);
                                            } else {
                                                let st = buffer.remove(cursor_line as usize + 1);
                                                buffer[cursor_line as usize]
                                                    .insert_str(cursor_column as usize, &st);
                                            }
                                        }
                                        Key::Backspace => {
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                            if cursor_column > 0 {
                                                cursor_column -= 1;
                                                buffer[cursor_line as usize]
                                                    .remove(cursor_column as usize);
                                            } else {
                                                if cursor_line as usize > buffer.len() {
                                                    cursor_line -= 1;
                                                    cursor_column = 0;
                                                } else {
                                                    let st =
                                                        if (cursor_line as usize) < buffer.len() {
                                                            buffer.remove(cursor_line as usize)
                                                        } else {
                                                            "".to_owned()
                                                        };
                                                    let length =
                                                        buffer[cursor_line as usize - 1].len();
                                                    buffer[cursor_line as usize - 1]
                                                        .insert_str(length, &st);
                                                    cursor_line -= 1;
                                                    cursor_column = length as isize;
                                                }
                                            }
                                        }
                                        Key::Char('k') => {
                                            if cursor_column as usize
                                                >= buffer[cursor_line as usize].len()
                                            {
                                                while cursor_column as usize
                                                    > buffer[cursor_line as usize].len()
                                                {
                                                    buffer[cursor_line as usize].push(' ');
                                                }
                                                let st = buffer.remove(cursor_line as usize + 1);
                                                buffer[cursor_line as usize]
                                                    .insert_str(cursor_column as usize, &st);
                                            //Copy pasta
                                            } else {
                                                buffer[cursor_line as usize]
                                                    .truncate(cursor_column as usize);
                                                //Copy pasta
                                            }
                                        }
                                        Key::Esc => {
                                            cursor_column = current_indentation;
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                        }
                                        Key::Char('\t') => {
                                            last_cursor_flip = std::time::Instant::now();
                                            cursor_on = true;
                                            cursor_column += 4;
                                        }
                                        Key::Alt(c) => println!("Alt-{}", c),
                                        Key::Ctrl(c) => println!("Ctrl-{}", c),
                                        Key::Left => println!("<left>"),
                                        Key::Right => println!("<right>"),
                                        Key::Up => println!("<up>"),
                                        Key::Down => println!("<down>"),
                                        _ => println!("Other"),
                                    }
                                }
                            }
                            _ => (),
                        }
                    }

                    None => break,
                }
            }
            if cursor_line < 0 {
                cursor_line = 0;
            }
            if cursor_column < 0 {
                cursor_column = 0;
            }
            if window_start + height - 2 < cursor_line as usize {
                window_start = cursor_line as usize - (height - 2);
            } else if window_start > cursor_line as usize {
                window_start = cursor_line as usize;
            }

            use termion::cursor::Goto;
            use termion::cursor::Hide;
            use termion::cursor::*;

            ///RENDER
            render_buffer.clear();
            render_buffer.push_str(termion::cursor::Hide.as_ref());
            render_buffer.push_str(termion::clear::All.as_ref());

            let mut skips = 0;
            let mut index = 0;
            while index + skips
                < (height.min((buffer.len() as isize - window_start as isize).max(0) as usize))
                    .min(height - 1)
            {
                let line = &buffer[index + window_start as usize];

                render_buffer
                    .push_str(&termion::cursor::Goto(0, 1 + (index + skips) as u16).to_string());
                render_buffer.push_str(&format!("{}", index as usize + window_start));
                render_buffer.push_str(
                    &termion::cursor::Goto(window_padding as u16 - 2, 1 + (index + skips) as u16)
                        .to_string(),
                );
                render_buffer.push_str(":");
                render_buffer.push_str(
                    &termion::cursor::Goto(window_padding as u16, 1 + (index + skips) as u16)
                        .to_string(),
                );
                render_buffer.push_str(line);

                skips += line_wrap_count(line, width - window_padding + 1);

                index += 1;
            }

            match file_mode {
                FileMode::Edit => {
                    let file = file_path.to_str().unwrap();
                    render_buffer.push_str(
                        &termion::cursor::Goto(
                            (width as isize - file.len() as isize).max(0) as u16,
                            height as u16,
                        )
                        .to_string(),
                    );
                    render_buffer.push_str(file);
                    render_buffer.push_str(&termion::cursor::Goto(0, height as u16).to_string());
                    if insert_mode {
                        render_buffer.push_str("Insert Mode");
                    } else {
                        render_buffer.push_str("Move Mode");
                    }
                }
                FileMode::Open => {
                    render_buffer.push_str(&termion::cursor::Goto(0, height as u16).to_string());
                    render_buffer.push_str("Open File : ");
                    render_buffer.push_str(&bottom_bar_buffer);
                }
                FileMode::SaveAsPrompt => {
                    render_buffer.push_str(&termion::cursor::Goto(0, height as u16).to_string());
                    render_buffer.push_str("Save as : ");
                    render_buffer.push_str(&bottom_bar_buffer);
                }
                _ => (),
            }

            if last_cursor_flip.elapsed().as_millis() > cursor_cycle_duration / 2 {
                last_cursor_flip = std::time::Instant::now();
                cursor_on = !cursor_on;
            }
            if cursor_on {
                render_buffer.push_str(
                    &termion::cursor::Goto(
                        (window_padding + (cursor_column as usize % (width - window_padding)))
                            as u16,
                        1 + (cursor_line as usize - window_start) as u16,
                    )
                    .to_string(),
                );
                render_buffer.push_str(termion::cursor::Show.as_ref());
            }

            write!(stdout, "{}", &render_buffer);
            stdout.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        write!(stdout, "{}", termion::cursor::Show);
        write!(stdout, "{}", termion::clear::All);
        write!(stdout, "{}", termion::cursor::Goto(1, 1));
    }
    println!("Thank you for using dte!");
}
