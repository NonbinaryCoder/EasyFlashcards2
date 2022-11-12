#[macro_export]
macro_rules! up {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Up,
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('w'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('W'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('k'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('K'),
            ..
        })
    };
}

#[macro_export]
macro_rules! down {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Down,
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('s'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('S'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('J'),
            ..
        })
    };
}

#[macro_export]
macro_rules! left {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Left,
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('a'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('A'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('h'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('H'),
            ..
        })
    };
}

#[macro_export]
macro_rules! right {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Right,
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('d'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('D'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('l'),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('L'),
            ..
        })
    };
}

#[macro_export]
macro_rules! click {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char(' '),
            ..
        }) | crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Enter,
            ..
        })
    };
}
