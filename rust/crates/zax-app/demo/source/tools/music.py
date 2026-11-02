"""Builds the full song from music/partition.json ("Saut de puce", the 8-bar theme) and writes
src/music_data.inc for src/music.s.

python tools/music.py

The theme alone loops every 14 seconds; the song extends it, in the same style (detached notes,
melody on pulse 1, punctuation on pulse 2, root-fifth bass on the triangle, short noise hits):
  A      the theme as it is (8 bars, C C F G);
  A'     the theme, its second half rewritten, pulse 2 answering on the off-beats, a walking
         bass at the phrase ends, more hits (8 bars);
  B      a bridge on A minor, F, C, G, a more singing melody (8 bars);
  break  long bass notes, arpeggios on pulse 2, eighth-note hits, a climb back to the theme
         (4 bars, F F G G);
then back to A: 28 bars, about 49 seconds.

Each channel gets its notes in time order: the step (sixteenth, two bytes) it starts on, its
length in frames, and its two period bytes (pulse and triangle: the timer's low byte, then its
high three bits with a length load; noise: the period index and $00). The tempo is the driver's
accumulator (136 BPM on 60.0988 frames per second, NTSC).
"""
import json

NTSC_CPU = 1789773
FPS = 60.0988
SCORE = json.load(open('music/partition.json'))
STEP_FRAMES = 60 / SCORE['bpm'] / SCORE['steps_per_beat'] * FPS
BAR = 16
# The noise period index nearest the reference's LFSR rate (one shift every 12 samples at
# 44.1 kHz): 3675 Hz, the NTSC period 508 (index 11).
NOISE_PERIOD = 0x0B
HIT = 0.045  # seconds

NOTE = {name: i for i, name in enumerate('C C# D D# E F F# G G# A A# B'.split())}


def m(name):
    """MIDI number of 'E5', 'A#3'..."""
    return NOTE[name[:-1]] + 12 * (int(name[-1]) + 1)


# Chords: (bass root, fifth, pulse 2 punctuation, the off-beat answers of A')
CHORDS = {
    'C': ('C3', 'G3', 'E4', ('E4', 'G4')),
    'F': ('F2', 'C3', 'A3', ('A3', 'C4')),
    'G': ('G2', 'D3', 'B3', ('B3', 'D4')),
    'Am': ('A2', 'E3', 'C4', ('C4', 'E4')),
}


class Song:
    def __init__(self):
        self.events = []

    def note(self, channel, step, length, midi):
        self.events.append({'channel': channel, 'start_step': step, 'duration_steps': length,
                            'midi': midi})

    def hit(self, step):
        self.events.append({'channel': 'noise', 'start_step': step, 'duration_seconds': HIT})

    def melody(self, bar, notes):
        """notes: (step in the bar, length in steps, 'E5')."""
        for step, length, name in notes:
            self.note('pulse1', bar * BAR + step, length, m(name))

    def bass(self, bar, chord, walk=None):
        root, fifth = CHORDS[chord][:2]
        names = walk or (root, fifth, root, fifth)
        for i, name in enumerate(names):
            self.note('triangle', bar * BAR + 4 * i, 2.3, m(name))


def theme(song, start_bar):
    """The 8 bars of partition.json, as they are."""
    for event in SCORE['events']:
        event = dict(event)
        event['start_step'] += start_bar * BAR
        song.events.append(event)


def theme_variation(song, b):
    chords = ['C', 'C', 'F', 'G', 'C', 'C', 'F', 'G']
    # the first half as the theme, with a pickup into the next bar
    first = [
        [(0, 2.55, 'E5'), (3, 2.55, 'G5'), (6, 2.8, 'E5'), (10, 1.55, 'C5'), (12, 2.8, 'D5')],
        [(0, 2.55, 'E5'), (3, 2.55, 'G5'), (6, 2.8, 'C6'), (10, 1.55, 'G5'), (12, 2.8, 'E5')],
        [(0, 2.55, 'F5'), (3, 2.55, 'A5'), (6, 2.8, 'F5'), (10, 1.55, 'E5'), (12, 1.55, 'C5'),
         (14, 1.3, 'C5')],
        [(0, 2.55, 'D5'), (3, 2.55, 'G5'), (6, 2.8, 'F5'), (10, 1.55, 'D5'), (12, 1.55, 'B4'),
         (14, 1.3, 'D5')],
    ]
    # the second half rewritten: a run up, a high answer, a turn back down
    second = [
        [(0, 1.55, 'E5'), (2, 1.55, 'G5'), (4, 1.55, 'A5'), (6, 1.55, 'G5'), (8, 1.55, 'E5'),
         (10, 1.55, 'D5'), (12, 2.8, 'C5')],
        [(0, 2.55, 'C6'), (3, 2.55, 'B5'), (6, 2.8, 'A5'), (10, 1.55, 'G5'), (12, 2.8, 'E5')],
        [(0, 2.55, 'F5'), (3, 2.55, 'A5'), (6, 2.8, 'C6'), (10, 1.55, 'A5'), (12, 2.8, 'F5')],
        [(0, 1.55, 'D5'), (2, 1.55, 'E5'), (4, 1.55, 'F5'), (6, 2.8, 'G5'), (10, 1.55, 'B4'),
         (12, 3.3, 'D5')],
    ]
    for i, notes in enumerate(first + second):
        bar = b + i
        song.melody(bar, notes)
        chord = chords[i]
        low, high = CHORDS[chord][3]
        for j, step in enumerate((2, 6, 10, 14)):
            song.note('pulse2', bar * BAR + step, 1.0, m(low if j % 2 == 0 else high))
        walk = ('G2', 'A2', 'B2', 'D3') if i in (3, 7) else None
        song.bass(bar, chord, walk)
        for step in (4, 10, 12) if i % 2 == 0 else (4, 12, 14):
            song.hit(bar * BAR + step)


def bridge(song, b):
    chords = ['Am', 'Am', 'F', 'F', 'C', 'C', 'G', 'G']
    melody = [
        [(0, 2.8, 'A5'), (3, 2.55, 'C6'), (6, 1.55, 'B5'), (8, 2.8, 'A5'), (12, 3.5, 'E5')],
        [(0, 1.55, 'C5'), (2, 1.55, 'E5'), (4, 5.5, 'A5'), (10, 1.55, 'G5'), (12, 3.5, 'A5')],
        [(0, 2.8, 'F5'), (3, 2.55, 'A5'), (6, 2.8, 'C6'), (10, 1.55, 'A5'), (12, 2.8, 'F5')],
        [(0, 1.55, 'E5'), (2, 1.55, 'F5'), (4, 2.8, 'A5'), (8, 5.5, 'G5')],
        [(0, 2.55, 'E5'), (3, 2.55, 'G5'), (6, 5.5, 'C6'), (12, 1.55, 'B5'), (14, 1.55, 'A5')],
        [(0, 2.8, 'G5'), (4, 2.8, 'E5'), (8, 2.8, 'C5'), (12, 2.8, 'E5')],
        [(0, 2.55, 'D5'), (3, 2.55, 'G5'), (6, 2.8, 'B5'), (10, 1.55, 'A5'), (12, 2.8, 'G5')],
        [(0, 2.8, 'F5'), (4, 1.55, 'E5'), (6, 2.8, 'D5'), (10, 1.55, 'B4'), (12, 3.3, 'D5')],
    ]
    for i, notes in enumerate(melody):
        bar = b + i
        song.melody(bar, notes)
        chord = chords[i]
        third = CHORDS[chord][2]
        for step in (6, 14):
            song.note('pulse2', bar * BAR + step, 1.25, m(third))
        song.bass(bar, chord)
        for step in (4, 12) if i % 2 == 0 else (4, 12, 14):
            song.hit(bar * BAR + step)


def pause(song, b):
    chords = ['F', 'F', 'G', 'G']
    arpeggios = {'F': ('A3', 'C4', 'F4', 'C4'), 'G': ('B3', 'D4', 'G4', 'D4')}
    for i, chord in enumerate(chords):
        bar = b + i
        root, fifth = CHORDS[chord][:2]
        song.note('triangle', bar * BAR, 6.5, m(root))
        song.note('triangle', bar * BAR + 8, 6.5, m(fifth))
        notes = arpeggios[chord]
        for j in range(8):
            song.note('pulse2', bar * BAR + 2 * j, 1.2, m(notes[j % 4]))
        for j in range(8):
            song.hit(bar * BAR + 2 * j)
    # calls on the melody, then a climb back to the theme's first E
    song.melody(b + 1, [(12, 1.55, 'A5'), (14, 1.55, 'G5')])
    song.melody(b + 3, [(8, 1.55, 'D5'), (10, 1.55, 'E5'), (12, 1.55, 'F5'), (14, 1.55, 'G5')])


def build():
    song = Song()
    theme(song, 0)
    theme_variation(song, 8)
    bridge(song, 16)
    pause(song, 24)
    return song.events, 28 * BAR


def frequency(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def timer(midi, divider):
    return round(NTSC_CPU / (divider * frequency(midi)) - 1)


def main():
    events, steps = build()
    lines = ['; Written by tools/music.py from music/partition.json ("Saut de puce", extended).',
             '', f'MUSIC_STEPS = {steps}',
             '; frames per step x 256 (136 BPM, NTSC): the driver\'s accumulator',
             f'MUSIC_STEP_FRAMES_256 = {round(STEP_FRAMES * 256)}', '']
    for channel in ('pulse1', 'pulse2', 'triangle', 'noise'):
        notes = sorted((e for e in events if e['channel'] == channel),
                       key=lambda e: e['start_step'])
        starts = [e['start_step'] for e in notes]
        assert len(starts) == len(set(starts)) and len(notes) < 256, channel
        frames, low, high = [], [], []
        for event in notes:
            if channel == 'noise':
                length = event['duration_seconds'] * FPS
                lo, hi = NOISE_PERIOD, 0
            else:
                length = event['duration_steps'] * STEP_FRAMES
                period = timer(event['midi'], 32 if channel == 'triangle' else 16)
                assert 8 <= period < 0x800, (channel, event)
                lo, hi = period & 0xFF, (period >> 8) | 0x08
            frames.append(max(1, round(length)))
            low.append(lo)
            high.append(hi)
        name = channel.capitalize()
        lines.append(f'; {channel}: {len(notes)} notes')
        lines.append(f'{name}Count = {len(notes)}')
        for label, values in (('StepLo', [s & 255 for s in starts]),
                              ('StepHi', [s >> 8 for s in starts]),
                              ('Frames', frames), ('Low', low), ('High', high)):
            lines.append(f'{name}{label}: .byte ' + ', '.join(f'${v:02X}' for v in values))
        lines.append('')
    open('src/music_data.inc', 'w').write('\n'.join(lines))
    print(f'{steps} steps, {steps * STEP_FRAMES / FPS:.1f} s;',
          ', '.join(f'{c} {sum(e["channel"] == c for e in events)}'
                    for c in ('pulse1', 'pulse2', 'triangle', 'noise')))


main()
