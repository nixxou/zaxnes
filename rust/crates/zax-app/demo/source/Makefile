# Collect all .s files in src/
SRC := $(wildcard src/*.s)

# Turn src/foo.s → src/foo.o
OBJ := $(SRC:.s=.o)

# Name of the final ROM
OUTPUT := FlappyZaxNes.nes

all: $(OUTPUT)

# Assemble each .s file to .o
src/%.o: src/%.s
	ca65 -g $< -o $@ -t nes

# Link all object files into a .nes file
$(OUTPUT): $(OBJ)
	ld65 $(OBJ) -o $(OUTPUT) -t nes -Ln build.lbl

chr:
	free-chr assets/spritesheet.png char.chr

clean:
	rm -f src/*.o $(OUTPUT)
