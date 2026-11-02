# Flappy NES

![Screenshot of the game](screenshot.png)

Esse é um jogo _homebrew_ baseado no Flappy Bird para o Nintendo Entertainment System (NES).

Para compilar, instale o [cc65](https://github.com/cc65/cc65). Além disso, tenha certeza que tem o GNU Make instalado. Daí, apenas digite `make` no terminal.

Caso queira modificar os sprites, o Makefile utiliza o programa [free-chr](https://github.com/filipemd/free-chr), daí é só mudar o arquivo `assets/spritesheet.png` e rodar `make chr`. Entretanto, você pode usar qualquer outro programa para essa tarefa, como o YY-CHR.