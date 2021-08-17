'''Combine all images into one texture atlas'''
import os
from PIL import Image, ImageOps

SIZE = 64
BLOCKS = {
    'debug': 0,
    'dirt': 1,
    'grass': 2,
    'grass_side': 3,
    'cobble': 4,
    'planks': 5,
    'sand': 6,
    'bricks': 7,
    'gravel': 8,
    'leaves': 9,
    'wood_top': 10,
    'wood': 11,
}
N = len(BLOCKS)

atlas = Image.new('RGBA', (SIZE, SIZE * N))
for block, id in BLOCKS.items():
    img = Image.open(block + '.png')
    atlas.paste(img, (0, SIZE * id))

atlas.save('atlas.png')
