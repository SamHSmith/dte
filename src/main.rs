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

fn dvorak_to_qwerty(c: char) -> char {
    #[cfg(feature = "qwerty")]
    {
        return match c {
            'u' => 'f',
            'e' => 'd',
            'w' => 'm',
            't' => 'k',
            'h' => 'j',
            'E' => 'D',
            'U' => 'F',
            'H' => 'J',
            'T' => 'K',
            'i' => 'g',
            'd' => 'h',
            'k' => 'c',
            'f' => 'y',
            'l' => 'p',
            other => other,
        };
    }
    c
}
fn srgb_lin(v: u8) -> u8 {
    let mut varR = v as f32 / 255.0;
    if (varR > 0.0031308) {
        varR = (1.055 * (varR + 0.055)).powf(2.4);
    } else {
        varR = varR / 12.92;
    }
    return (varR * 255.0) as u8;
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
        if should_load_file && !file_path.is_file() {
            //It's a new file.
            should_load_file = false;
        }

        //HIGHLIGHTING
        use syntect::easy::HighlightLines;
        use syntect::highlighting::{Style, ThemeSet};
        use syntect::parsing::SyntaxSet;
        use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

        // Load these once at the start of your program
        let ps = SyntaxSet::load_defaults_nonewlines();
        let ts = ThemeSet::load_defaults();

        let new_hl = |path: &std::path::Path| HighlightLines::new(
                ps.find_syntax_by_extension(path.extension()
                    .unwrap_or(std::ffi::OsStr::new("")).to_str().unwrap())
                    .unwrap_or(ps.find_syntax_plain_text()),
                &ts.themes["base16-eighties.dark"],
            );

        let mut render_buffer: String = String::new();
        let mut width: usize; let mut last_width = 0;
        let mut height: usize; let mut last_height = 0;
        {
            let (w, h) = termion::terminal_size().unwrap();
            width = w as usize;
            height = h as usize;
        }
        let mut cursor_line: isize = 0;
        let mut cursor_column: isize = 0;
        let mut window_padding = 0; //Gets set dynamically
        let mut show_line_nums = true;
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

        let mut first_loop = true;
        while running {
            {
                let (w, h) = termion::terminal_size().unwrap();
                width = w as usize;
                height = h as usize;
            }
            let mut should_render = false;
            if first_loop {
                should_render = true;
            }
            if width != last_width || height != last_height {
                should_render = true;
            }
            last_width = width;
            last_height = height;

            if should_load_file {
                should_render = true;
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
                        Ok(0) => {
                            buffer.pop();
                            break;
                            },
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
                        should_render = true;
                        match file_mode {
                            FileMode::Open => match c.unwrap() {
                                Key::Char(c) if c == (dvorak_to_qwerty('\n')) => {
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
                                Key::Char(c) if c == (dvorak_to_qwerty('\n')) => {
                                    let path = std::path::Path::new(&bottom_bar_buffer);
                                    if !path.is_dir() {
                                        use std::fs::OpenOptions;
                                        use std::io::{BufWriter, Write};
                                        let mut f = OpenOptions::new()
                                            .read(true)
                                            .write(true)
                                            .create(true)
                                            .truncate(true)
                                            .open(path)
                                            .unwrap();

                                        for b in buffer.iter() {
                                            f.write_all(b.as_bytes()).unwrap();
                                            f.write_all(b"\n");
                                        }
                                        file_mode = FileMode::Edit;
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
                            FileMode::Edit => {
                                ///////////////
                                if insert_mode {
                                    match c.unwrap() {
                                        Key::Char(c) if c == (dvorak_to_qwerty('\n')) => {
                                            let newstr = buffer[cursor_line as usize]
                                                .split_off(cursor_column as usize);
                                            buffer.insert(cursor_line as usize + 1, newstr);
                                            cursor_column = current_indentation;
                                            cursor_line += 1;
                                        }
                                        Key::Ctrl(c) if c == (dvorak_to_qwerty('t')) => {
                                            buffer[cursor_line as usize]
                                                .insert(cursor_column as usize, '\t');
                                            cursor_column += 1;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('\t')) => {
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
                                            if cursor_column > 0 {
                                                cursor_column -= 1;
                                                buffer[cursor_line as usize]
                                                    .remove(cursor_column as usize);
                                            } else if cursor_line > 0 {
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
                                        Key::Char(c) if c == (dvorak_to_qwerty('q')) => {
                                            running = false
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('l')) => {
                                            show_line_nums = !show_line_nums;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('f')) => {
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
                                        Key::Char(c) if c == (dvorak_to_qwerty('w')) => {
                                            file_mode = FileMode::SaveAsPrompt;
                                            bottom_bar_buffer.clear();
                                            bottom_bar_buffer.insert_str(
                                                0,
                                                file_path.as_path().to_str().unwrap(),
                                            );
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('t')) => {
                                            cursor_column += 1;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('h')) => {
                                            cursor_column -= 1;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('e')) => {
                                            cursor_line += 1;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('u')) => {
                                            cursor_line -= 1;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('E')) => {
                                            cursor_line += 20;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('U')) => {
                                            cursor_line -= 20;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('i')) => {
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
                                        Key::Char(c) if c == (dvorak_to_qwerty('d')) => {
                                            if (cursor_line as usize) < buffer.len()
                                                && (cursor_column as usize)
                                                    < buffer[cursor_line as usize].len()
                                            {
                                                buffer[cursor_line as usize]
                                                    .remove(cursor_column as usize);
                                            } else if (cursor_line as usize + 1) < buffer.len() {
                                                let st = buffer.remove(cursor_line as usize + 1);
                                                buffer[cursor_line as usize]
                                                    .insert_str(cursor_column as usize, &st);
                                            }
                                        }
                                        Key::Backspace => {
                                            if cursor_column > 0 {
                                                cursor_column -= 1;
                                                buffer[cursor_line as usize]
                                                    .remove(cursor_column as usize);
                                            } else {
                                                if cursor_line as usize > buffer.len() {
                                                    cursor_line -= 1;
                                                    cursor_column = 0;
                                                } else if cursor_line > 0 {
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
                                        Key::Char(c) if c == (dvorak_to_qwerty('k')) => {
                                            if (cursor_line as usize) < buffer.len() {
                                                if cursor_column as usize
                                                    >= buffer[cursor_line as usize].len()
                                                    && buffer.len() > (cursor_line + 1) as usize
                                                {
                                                    while cursor_column as usize
                                                        > buffer[cursor_line as usize].len()
                                                    {
                                                        buffer[cursor_line as usize].push(' ');
                                                    }
                                                    let st =
                                                        buffer.remove(cursor_line as usize + 1);
                                                    buffer[cursor_line as usize]
                                                        .insert_str(cursor_column as usize, &st);
                                                //Copy pasta
                                                } else {
                                                    buffer[cursor_line as usize]
                                                        .truncate(cursor_column as usize);
                                                    //Copy pasta
                                                }
                                            }
                                        }
                                        Key::Esc => {
                                            cursor_column = current_indentation;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('\t')) => {
                                            cursor_column += 4;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('T')) => {
                                            cursor_column += 4;
                                        }
                                        Key::Char(c) if c == (dvorak_to_qwerty('H')) => {
                                            cursor_column -= 4;
                                        }
                                        _ => (),
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
            if window_start + height - 2 <= cursor_line as usize {
                window_start = cursor_line as usize - (height - 2);
            } else if window_start > cursor_line as usize {
                window_start = cursor_line as usize;
            }

            if show_line_nums {
                window_padding = 6;
            } else {
                window_padding = 0;
            }

            use termion::cursor::Goto;
            use termion::cursor::Hide;
            use termion::cursor::*;

            ///RENDER
            let mut highlight_buffer = String::new();

            if should_render {
                render_buffer.clear();
                render_buffer.push_str(termion::cursor::Hide.as_ref());
                if !show_line_nums {
                    render_buffer.push_str(termion::clear::All.as_ref());
                }

                let mut skips = 0;
                let mut skips_before_cursor = 0;

                let mut h = new_hl(&file_path);
                
                for i in 0..window_start.min(buffer.len()) {
                    h.highlight(&buffer[i], &ps);
                }

                let mut index = 0;
                while index
                    < ((height - skips)
                        .min((buffer.len() as isize - window_start as isize).max(0) as usize))
                    .min((height - skips) - 1)
                {
                    let line = &buffer[index + window_start as usize];

                    highlight_buffer.clear();
                    let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                    for (s, t) in ranges {
                        let r = srgb_lin(s.foreground.r);
                        let g = srgb_lin(s.foreground.g);
                        let b = srgb_lin(s.foreground.b);

                        highlight_buffer.push_str(&termion::color::Rgb(r, g, b).fg_string());

                        highlight_buffer.push_str(t);
                    }
                    let highlightedLine = &highlight_buffer;

                    render_buffer.push_str(
                        &termion::cursor::Goto(0, 1 + (index + skips) as u16).to_string(),
                    );
                    for i in 0..window_padding {
                        render_buffer.push(' ');
                    }
                    if show_line_nums {
                        render_buffer.push_str(
                            &termion::cursor::Goto(0, 1 + (index + skips) as u16).to_string(),
                        );
                        render_buffer.push_str(&format!("{}", index as usize + window_start + 1));
                        render_buffer.push_str(
                            &termion::cursor::Goto(
                                window_padding as u16 - 2,
                                1 + (index + skips) as u16,
                            )
                            .to_string(),
                        );
                        render_buffer.push_str(":");
                    }
                    render_buffer.push_str(
                        &termion::cursor::Goto(window_padding as u16, 1 + (index + skips) as u16)
                            .to_string(),
                    );

                    render_buffer.push_str(&highlightedLine.replace("\t", "    "));
                    render_buffer.push_str(termion::color::Reset {}.fg_str());
                    render_buffer.push_str(termion::color::Reset {}.bg_str());

                    let line_wraps = line_wrap_count(line, width - window_padding + 1);
                    skips += line_wraps;
                    if index + window_start < cursor_line as usize {
                        skips_before_cursor = skips;
                    }

                    let columns_left = if line_wraps <= 0 {
                        (width as isize - window_padding as isize - line.len() as isize).max(0)
                            as usize
                    } else {
                        width - (line.len() % (width - window_padding))
                    };
                    if show_line_nums {
                        for i in 0..columns_left + 1 {
                            //For some reason the maths is 1 short
                            render_buffer.push(' ');
                        }
                    }

                    index += 1;
                }
                render_buffer
                    .push_str(&termion::cursor::Goto(1, 1 + (index + skips) as u16).to_string());
                render_buffer.push_str(termion::clear::AfterCursor.as_ref());

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
                        render_buffer
                            .push_str(&termion::cursor::Goto(0, height as u16).to_string());
                        if insert_mode {
                            render_buffer.push_str("Insert Mode");
                        } else {
                            render_buffer.push_str("Move Mode");
                        }
                    }
                    FileMode::Open => {
                        render_buffer
                            .push_str(&termion::cursor::Goto(0, height as u16).to_string());
                        render_buffer.push_str("Open File : ");
                        render_buffer.push_str(&bottom_bar_buffer);
                    }
                    FileMode::SaveAsPrompt => {
                        render_buffer
                            .push_str(&termion::cursor::Goto(0, height as u16).to_string());
                        render_buffer.push_str("Save as : ");
                        render_buffer.push_str(&bottom_bar_buffer);
                    }
                    _ => (),
                }
                let mut tab_count = 0;
                if (cursor_line as usize) < buffer.len() {
                    for i in 0..cursor_column {
                        if (i as usize) < (buffer[cursor_line as usize]).len() {
                            if &(buffer[cursor_line as usize])[(i as usize)..(i as usize + 1)]
                                == "\t"
                            {
                                tab_count += 1;
                            }
                        }
                    }
                }
                render_buffer.push_str(
                    &termion::cursor::Goto(
                        (window_padding
                            + ((cursor_column + tab_count * 3) as usize % (width - window_padding)))
                            as u16,
                        1 + (cursor_line as usize - window_start + skips_before_cursor) as u16,
                    )
                    .to_string(),
                );
                render_buffer.push_str(termion::cursor::Show.as_ref());
                write!(stdout, "{}", &render_buffer);
                stdout.flush().unwrap();
            }
            first_loop = false;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        write!(stdout, "{}", termion::cursor::Show);
        write!(stdout, "{}", termion::clear::All);
        write!(stdout, "{}", termion::cursor::Goto(1, 1));
    }
    println!("Thank you for using dte!");
}


