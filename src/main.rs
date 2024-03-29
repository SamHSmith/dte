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
            let arg1 = args.remove(1);
            if arg1.chars().nth(0).unwrap_or(' ') == '/' {
                file_path = std::path::PathBuf::from(arg1);
            } else {
                file_path.push(arg1);
            }
        } else {
            should_load_file = false;
        }
        if should_load_file && !file_path.is_file() {
            //It's a new file.
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
        let mut window_padding: usize = 0; //Gets set dynamically
        let mut show_line_nums = true;
        let mut window_start = 0;
        let mut running = true;

        let mut current_indentation: usize = 0;

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
                        }
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
                                Key::Char(c) if c == '\n' => {
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
                                Key::Char(c) if c == '\n' => {
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
                                        Key::Char(c) if c == '\n' => {
                                            let mut tmp = 0;
                                            {
                                                let mut chars =
                                                    buffer[cursor_line as usize].chars();
                                                for i in 0..cursor_column {
                                                    tmp += chars.next().unwrap().len_utf8();
                                                }
                                            }

                                            let mut newstr =
                                                buffer[cursor_line as usize].split_off(tmp);
                                            for i in 0..current_indentation {
                                                newstr.insert(0, ' ');
                                            }
                                            buffer.insert(cursor_line as usize + 1, newstr);
                                            cursor_column = current_indentation as isize;
                                            cursor_line += 1;
                                        }
                                        Key::Char(c) if c == '\t' => {
                                            let pad_count = 4 - (cursor_column % 4);
                                            for x in 0..pad_count {
                                                let index = if cursor_column
                                                    < buffer[cursor_line as usize].chars().count()
                                                        as isize
                                                {
                                                    buffer[cursor_line as usize]
                                                        .char_indices()
                                                        .nth(cursor_column as usize)
                                                        .unwrap()
                                                        .0
                                                } else {
                                                    buffer[cursor_line as usize].len()
                                                };
                                                buffer[cursor_line as usize].insert(index, ' ');
                                                cursor_column += 1;
                                            }
                                        }
                                        Key::Ctrl(c) if c == 't' => {
                                            buffer[cursor_line as usize]
                                                .insert(cursor_column as usize, '\t');
                                            cursor_column += 1;
                                        }
                                        Key::Char(c) if c == '}' => {
                                            if cursor_column >= 4
                                                && buffer[cursor_line as usize]
                                                    .chars()
                                                    .nth(cursor_column as usize - 1)
                                                    .unwrap()
                                                    == ' '
                                                && buffer[cursor_line as usize]
                                                    .chars()
                                                    .nth(cursor_column as usize - 2)
                                                    .unwrap()
                                                    == ' '
                                                && buffer[cursor_line as usize]
                                                    .chars()
                                                    .nth(cursor_column as usize - 3)
                                                    .unwrap()
                                                    == ' '
                                                && buffer[cursor_line as usize]
                                                    .chars()
                                                    .nth(cursor_column as usize - 4)
                                                    .unwrap()
                                                    == ' '
                                            {
                                                buffer[cursor_line as usize].replace_range(
                                                    ((cursor_column as usize) - 4)
                                                        ..(cursor_column as usize),
                                                    "",
                                                );
                                                cursor_column -= 4;
                                            }
                                            buffer[cursor_line as usize]
                                                .insert(cursor_column as usize, c);
                                            cursor_column += 1;
                                        }
                                        Key::Char(c) => {
                                            let index = if cursor_column
                                                < buffer[cursor_line as usize].chars().count()
                                                    as isize
                                            {
                                                buffer[cursor_line as usize]
                                                    .char_indices()
                                                    .nth(cursor_column as usize)
                                                    .unwrap()
                                                    .0
                                            } else {
                                                buffer[cursor_line as usize].len()
                                            };
                                            buffer[cursor_line as usize].insert(index, c);
                                            cursor_column += 1;
                                        }
                                        Key::Backspace => {
                                            if cursor_column > 0 {
                                                cursor_column -= 1;
                                                let mut tmp = 0;
                                                let mut tmp_size = 0;
                                                {
                                                    let mut chars =
                                                        buffer[cursor_line as usize].chars();
                                                    for i in 0..cursor_column {
                                                        tmp += chars.next().unwrap().len_utf8();
                                                    }
                                                    tmp_size = chars.next().unwrap().len_utf8();
                                                }
                                                buffer[cursor_line as usize]
                                                    .replace_range(tmp..(tmp + tmp_size), "");
                                            } else if cursor_line > 0 {
                                                let st = buffer.remove(cursor_line as usize);
                                                let length = buffer[cursor_line as usize - 1]
                                                    .chars()
                                                    .count();
                                                buffer[cursor_line as usize - 1].push_str(&st);
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
                                        Key::Char(c) if c == 'q' => running = false,
                                        Key::Char(c) if c == 'l' => {
                                            show_line_nums = !show_line_nums;
                                        }
                                        Key::Char(c) if c == 'f' => {
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
                                        Key::Char(c) if c == 'w' => {
                                            file_mode = FileMode::SaveAsPrompt;
                                            bottom_bar_buffer.clear();
                                            bottom_bar_buffer.insert_str(
                                                0,
                                                file_path.as_path().to_str().unwrap(),
                                            );
                                        }
                                        Key::Char(c) if c == 't' => {
                                            cursor_column += 1;
                                        }
                                        Key::Char(c) if c == 'h' => {
                                            cursor_column -= 1;
                                        }
                                        Key::Char(c) if c == 'e' => {
                                            cursor_line += 1;
                                        }
                                        Key::Char(c) if c == 'u' => {
                                            cursor_line -= 1;
                                        }
                                        Key::Char(c) if c == 'E' => {
                                            cursor_line += 20;
                                        }
                                        Key::Char(c) if c == 'U' => {
                                            cursor_line -= 20;
                                        }
                                        Key::Char(c) if c == 'i' => {
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
                                        Key::Char(c) if c == 'd' => {
                                            if (cursor_line as usize) < buffer.len()
                                                && (cursor_column as usize)
                                                    < buffer[cursor_line as usize].chars().count()
                                            {
                                                let mut tmp = 0;
                                                let mut tmp_size = 0;
                                                {
                                                    let mut chars =
                                                        buffer[cursor_line as usize].chars();
                                                    for i in 0..cursor_column {
                                                        tmp += chars.next().unwrap().len_utf8();
                                                    }
                                                    tmp_size = chars.next().unwrap().len_utf8();
                                                }
                                                buffer[cursor_line as usize]
                                                    .replace_range(tmp..(tmp + tmp_size), "");
                                            } else if (cursor_line as usize + 1) < buffer.len() {
                                                let st = buffer.remove(cursor_line as usize + 1);
                                                buffer[cursor_line as usize]
                                                    .insert_str(cursor_column as usize, &st);
                                            }
                                        }
                                        Key::Backspace => {
                                            if cursor_column > 0 {
                                                cursor_column -= 1;
                                                let mut tmp = 0;
                                                let mut tmp_size = 0;
                                                {
                                                    let mut chars =
                                                        buffer[cursor_line as usize].chars();
                                                    for i in 0..cursor_column {
                                                        tmp += chars.next().unwrap().len_utf8();
                                                    }
                                                    tmp_size = chars.next().unwrap().len_utf8();
                                                }
                                                buffer[cursor_line as usize]
                                                    .replace_range(tmp..(tmp + tmp_size), "");
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
                                        Key::Char(c) if c == 'k' => {
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
                                            cursor_column = 0;
                                        }
                                        Key::Char(c) if c == 'o' => {
                                            if (cursor_line as usize) < buffer.len() {
                                                cursor_column =
                                                    buffer[cursor_line as usize].chars().count()
                                                        as isize;
                                            } else {
                                                cursor_column = current_indentation as isize;
                                            }
                                        }
                                        Key::Char(c) if c == 'a' => {
                                            cursor_column = current_indentation as isize;
                                        }
                                        Key::Char(c) if c == '\t' => {
                                            cursor_column += 4;
                                        }
                                        Key::Char(c) if c == 'T' => {
                                            cursor_column += 4;
                                        }
                                        Key::Char(c) if c == 'H' => {
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
                window_padding = 3 + 1 + (f64::log10(buffer.len() as f64) as usize);
            } else {
                window_padding = 0;
            }

            if (cursor_line as usize) < buffer.len() {
                current_indentation = 0;
                for c in buffer[cursor_line as usize].chars() {
                    if c != ' ' {
                        break;
                    }
                    current_indentation += 1;
                }
            }

            use termion::cursor::Goto;
            use termion::cursor::Hide;
            use termion::cursor::*;

            ///RENDER
            if should_render {
                render_buffer.clear();
                render_buffer.push_str(termion::cursor::Hide.as_ref());
                if !show_line_nums {
                    render_buffer.push_str(termion::clear::All.as_ref());
                }

                let mut skips = 0;
                let mut skips_before_cursor = 0;

                let mut index = 0;
                while index
                    < ((height - skips)
                        .min((buffer.len() as isize - window_start as isize).max(0) as usize))
                    .min((height - skips) - 1)
                {
                    let line = &buffer[index + window_start as usize];

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

                    let mut remainder_are_spaces_index = line.chars().count();
                    while remainder_are_spaces_index > 0
                        && line.chars().nth(remainder_are_spaces_index - 1).unwrap() == ' '
                        && (remainder_are_spaces_index > cursor_column as usize
                            || (index + window_start) != cursor_line as usize)
                    {
                        remainder_are_spaces_index -= 1;
                    }
                    for (i, c) in line.chars().enumerate() {
                        if i >= remainder_are_spaces_index {
                            break;
                        }
                        render_buffer.push(c);
                    }
                    render_buffer.push_str(termion::style::Underline {}.as_ref());
                    for i in remainder_are_spaces_index..line.chars().count() {
                        render_buffer.push_str("%");
                    }
                    render_buffer.push_str(termion::style::NoUnderline {}.as_ref());

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
                        if (i as usize) < buffer[cursor_line as usize].chars().count() {
                            if buffer[cursor_line as usize]
                                .chars()
                                .nth(i as usize)
                                .unwrap()
                                == '\t'
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
