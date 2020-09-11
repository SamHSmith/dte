# dte
May I introduce you to a little project of mine, Dvorak Text Editor.

Dte, is the result of my frustration with emacs start times, and someone pushing an automatic update that broke my rust-mode indendation in emacs. I decided I wanted to write something new. Borrowing from the "suckless" philosophy I am aiming for something simple. Yet integrated enough that you shouldn't need to go pickup a bag full of plugins to make your editor usefull. The controls are vim "inspired" but because this is a dvorak text editor you move the cursor with the e, u, h, and t keys. **However there is a qwerty mode.** You just need to enable it at compile time. But I warn you, some of the key choices make sense on dvorak but on qwerty however, the key choices will seem totally arbetrary. But think of it like this, you pay a cost to learn the keys in the first place.(Small cost, because simple you know.) And once you have learned them you get the benifit of very ergonomic controls for the rest of eternity. (Now that's a bargain.)

## Controls
You start the editor by running "dte" in a terminal or tty. You can open a file from the command line with dte {path to file}.
### Dvorak
- f open the "Open file" menu. Esc is cancel and enter is open.
- u to move the cursor up
- e to move the cursor down
- h to move the cursor left
- t to move the cursor right
- backspace to remove characters
- d to delete characters
- k to delete all the characters on the current line after the cursor
- i to enter insert mode. In insert mode you type normally. To exit insert mode press Esc.
- Esc to move to the start of the current line. Works only while not in insert mode.(aka move mode)
- hold shift while pressing e and u to move ten lines at a time.
- hold shift while pressing h and t to move 4 columns at a time.
- Ctrl-t in insert mode to write 4 spaces
- w to write to a file. Works just like open file.
- l to toggle line numbers. The editor is copy/paste friendly when not showing line numbers.
- q to quit

### Qwerty
- y open the "Open file" menu. Esc is cancel and enter is open.
- f to move the cursor up
- d to move the cursor down
- j to move the cursor left
- k to move the cursor right
- backspace to remove characters
- h to delete characters
- c to delete all the characters on the current line after the cursor
- g to enter insert mode. In insert mode you type normally. To exit insert mode press Esc.
- Esc to move to the start of the current line. Works only while not in insert mode.(aka move mode)
- hold shift while pressing d and f to move ten lines at a time.
- hold shift while pressing j and k to move 4 columns at a time.
- Ctrl-k in insert mode to write 4 spaces
- m to write to a file. Works just like open file.
- p to toggle line numbers. The editor is copy/paste friendly when not showing line numbers.
- q to quit

## Deps
[The rust toolchain](https://www.rust-lang.org/tools/install).

## Installation for dvorak users (if you don't know what dvorak is skip to the qwerty section)

```bash
git clone https://github.com/SamHSmith/dte.git
cd dte
cargo build
sudo cp target/debug/dte /usr/bin
```

## Installation for qwerty users

```bash
git clone https://github.com/SamHSmith/dte.git
cd dte
cargo build --features="qwerty"
sudo cp target/debug/dte /usr/bin
```
