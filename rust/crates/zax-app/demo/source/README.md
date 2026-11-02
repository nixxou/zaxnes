# Flappy NES

![Screenshot of the game](screenshot.png)

This is a homebrew game based on Flappy Bird for the Nintendo Entertainment System (NES).

To compile it, install [cc65](https://github.com/cc65/cc65). Also make sure you have GNU Make installed. Then just type `make` in the terminal.

If you want to modify the sprites, the Makefile uses the program [free-chr](https://github.com/filipemd/free-chr). Just change the file `assets/spritesheet.png` and run `make chr`. However, you can use any other program for this task as well, such as YY-CHR.
