# dte
May I introduce you to a little project of mine, Dvorak Text Editor.

Dte, is the result of my frustration with emacs start times, and someone pushing an automatic update that broke my rust-mode indendation in emacs. I decided I wanted to write something new. Borrowing from the "suckless" philosophy I am aiming for something simple. Yet integrated enough that you shouldn't need to go pickup a bag full of plugins to make your editor usefull. The controls are vim "inspired" but because this is a dvorak text editor you move the cursor with the e, u, h, and t keys. **However there is a qwerty mode.** You just need to enable it at compile time. But I warn you, some of the key choices make sense on dvorak but on qwerty however, the key choices will seem totally arbetrary. But think of it like this, you pay a cost to learn the keys in the first place.(Small cost, because simple you know.) And once you have learned them you get the benifit of very ergonomic controls for the rest of eternity. (Now that's a bargain.)

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
