#!/usr/bin/env python3
"""Headless integration checks; use --self-test to test the PNG reader alone."""
import argparse
import hashlib
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib

SIGNATURE = b'\x89PNG\r\n\x1a\n'
BODIES = ('sol', 'rocoso', 'gaseoso', 'oceanico', 'volcanico', 'helado')
DEFAULT_EXE = Path(__file__).resolve().parents[1] / 'target/debug/lab5-graficas.exe'


def decode_png(data):
    """Decode the RGB8, non-interlaced, filter-0 PNG produced by src/png.rs."""
    if not data.startswith(SIGNATURE):
        raise ValueError('invalid PNG signature')
    pos = len(SIGNATURE)
    chunks = []
    idat = []
    ended = False
    while pos < len(data):
        if pos + 12 > len(data):
            raise ValueError('truncated chunk')
        length = struct.unpack_from('>I', data, pos)[0]
        end = pos + 12 + length
        if end > len(data):
            raise ValueError('truncated chunk data')
        kind = data[pos + 4:pos + 8]
        payload = data[pos + 8:pos + 8 + length]
        crc = struct.unpack_from('>I', data, pos + 8 + length)[0]
        if zlib.crc32(kind + payload) & 0xffffffff != crc:
            raise ValueError('chunk CRC mismatch')
        chunks.append(kind)
        if kind == b'IHDR':
            if len(chunks) != 1 or length != 13:
                raise ValueError('invalid IHDR')
            width, height, depth, color, compression, filtering, interlace = struct.unpack('>IIBBBBB', payload)
            if not (1 <= width <= 4096 and 1 <= height <= 4096):
                raise ValueError('unsafe or invalid dimensions')
            if (depth, color, compression, filtering, interlace) != (8, 2, 0, 0, 0):
                raise ValueError('expected non-interlaced RGB8 PNG')
        elif kind == b'IDAT':
            idat.append(payload)
        elif kind == b'IEND':
            if length or end != len(data):
                raise ValueError('invalid IEND or trailing data')
            ended = True
            break
        pos = end
    if not ended or not chunks or chunks[0] != b'IHDR' or not idat:
        raise ValueError('missing required PNG chunks')
    if chunks.count(b'IHDR') != 1:
        raise ValueError('duplicate IHDR')
    size = height * (1 + width * 3)
    try:
        decoder = zlib.decompressobj()
        raw = decoder.decompress(b''.join(idat), size + 1)
        if len(raw) != size or not decoder.eof or decoder.unused_data or decoder.unconsumed_tail:
            raise ValueError('invalid decompressed length or zlib stream')
    except zlib.error as exc:
        raise ValueError('invalid zlib stream') from exc
    stride = width * 3 + 1
    rows = []
    for offset in range(0, size, stride):
        if raw[offset] != 0:
            raise ValueError('unsupported PNG row filter')
        rows.append(raw[offset + 1:offset + stride])
    return width, height, b''.join(rows)


def chunk(kind, payload):
    return struct.pack('>I', len(payload)) + kind + payload + struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff)


class DecoderTests(unittest.TestCase):
    def png(self, raw=b'\x00\xff\x00\x00\x00\xff\x00'):
        header = struct.pack('>IIBBBBB', 2, 1, 8, 2, 0, 0, 0)
        return SIGNATURE + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')

    def test_known_rgb(self):
        self.assertEqual(decode_png(self.png()), (2, 1, b'\xff\x00\x00\x00\xff\x00'))

    def test_bad_signature(self):
        with self.assertRaises(ValueError):
            decode_png(b'bad')

    def test_bad_crc(self):
        data = bytearray(self.png())
        data[29] ^= 1
        with self.assertRaisesRegex(ValueError, 'CRC'):
            decode_png(data)

    def test_filter_and_truncation(self):
        with self.assertRaisesRegex(ValueError, 'filter'):
            decode_png(self.png(b'\x01\xff\x00\x00\x00\xff\x00'))
        with self.assertRaises(ValueError):
            decode_png(self.png()[:-1])

    def test_gallery_detects_sun_only_and_duplicate_panels(self):
        def write_rgb(path, width, height, pixel):
            header = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
            raw = b''.join(b'\0' + b''.join(pixel(x, y) for x in range(width))
                           for y in range(height))
            path.write_bytes(SIGNATURE + chunk(b'IHDR', header) +
                             chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))

        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR') or None) as tmp:
            root = Path(tmp)
            def color(body, x, y):
                return bytes((body * 35, x % 100, y % 100))
            for index, body in enumerate(BODIES):
                write_rgb(root / (body + '.png'), 100, 100,
                          lambda x, y, index=index: color(index, x, y))
            gallery = root / 'gallery.png'
            write_rgb(gallery, 300, 200, lambda x, y: color((y // 100) * 3 + x // 100, x % 100, y % 100))
            check_gallery(gallery)
            write_rgb(gallery, 300, 200, lambda x, y: color(0, x % 100, y % 100))
            with self.assertRaises(AssertionError):
                check_gallery(gallery)
            write_rgb(gallery, 300, 200, lambda x, y: color(0, x // 3, y // 2))
            with self.assertRaises(AssertionError):
                check_gallery(gallery)


class Results:
    def __init__(self):
        self.passed = 0
        self.failed = 0

    def check(self, label, fn):
        try:
            fn()
        except (AssertionError, ValueError, OSError, subprocess.SubprocessError) as exc:
            self.failed += 1
            print('FAIL {}: {}'.format(label, exc), flush=True)
            return False
        self.passed += 1
        print('PASS {}'.format(label), flush=True)
        return True


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def invoke(exe, cwd, *args):
    try:
        return subprocess.run([str(exe), *args], cwd=str(cwd), stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=90, check=False)
    except subprocess.TimeoutExpired as exc:
        raise AssertionError('timed out after 90 seconds') from exc


def succeeded(result):
    require(result.returncode == 0, 'exit {}: {}'.format(
        result.returncode, result.stderr.decode('utf-8', errors='replace')[-500:]))


def image(path):
    require(path.is_file(), 'missing output: {}'.format(path))
    data = path.read_bytes()
    width, height, pixels = decode_png(data)
    require(64 <= width <= 4096 and 64 <= height <= 4096,
            'unrealistic dimensions: {}x{}'.format(width, height))
    # A small set is enough to distinguish a solid fill without large allocations.
    colors = set()
    for i in range(0, len(pixels), 3):
        colors.add(pixels[i:i + 3])
        if len(colors) >= 16:
            break
    require(len(colors) >= 16, 'image lacks color diversity')
    # Check the central surface, not just background stars or a gradient.
    # Every preset places the planet over this central 96x96 square.
    center_colors = set()
    bright = 0
    samples = 0
    for y in range(height // 2 - 48, height // 2 + 48):
        for x in range(width // 2 - 48, width // 2 + 48):
            offset = (y * width + x) * 3
            pixel = pixels[offset:offset + 3]
            center_colors.add(pixel)
            bright += max(pixel) >= 28
            samples += 1
    require(len(center_colors) >= 128, 'central body lacks shader detail')
    require(bright > samples // 4, 'central body is missing or nearly black')
    return hashlib.sha256(pixels).hexdigest()


def check_gallery(path):
    require(path.is_file(), 'missing gallery')
    width, height, pixels = decode_png(path.read_bytes())
    require(width % 3 == 0 and height % 2 == 0, 'invalid gallery grid dimensions')
    # Compare every pixel of each full tile, not just its central colors.
    signatures = []
    for row in range(2):
        for col in range(3):
            panel = bytearray()
            colors = set()
            body = BODIES[row * 3 + col]
            body_path = path.parent / (body + '.png')
            require(body_path.is_file(), 'missing source image for ' + body)
            bw, bh, body_pixels = decode_png(body_path.read_bytes())
            require((width, height) == (bw * 3, bh * 2), 'gallery tile dimensions do not match ' + body)
            for y in range(bh):
                start = ((row * bh + y) * width + col * bw) * 3
                require(pixels[start:start + bw * 3] == body_pixels[y * bw * 3:(y + 1) * bw * 3],
                        'gallery tile does not match ' + body)
            body_colors = set()
            for by in range(int(bh * 0.3), int(bh * 0.7)):
                for bx in range(int(bw * 0.3), int(bw * 0.7)):
                    index = (by * bw + bx) * 3
                    body_colors.add(body_pixels[index:index + 3])
            matches = 0
            for iy in range(12):
                y = int((row + 0.35 + 0.3 * iy / 11) * height / 2)
                for ix in range(12):
                    x = int((col + 0.35 + 0.3 * ix / 11) * width / 3)
                    offset = (y * width + x) * 3
                    pixel = pixels[offset:offset + 3]
                    colors.add(pixel)
                    panel.extend(pixel)
                    matches += pixel in body_colors
            require(len(colors) >= 16, 'gallery panel {} lacks detail'.format(len(signatures) + 1))
            require(matches >= 72, 'gallery panel does not match ' + body)
            signatures.append(bytes(panel))
    require(len(set(signatures)) == 6, 'gallery contains duplicate panels')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', nargs='?', type=Path, default=DEFAULT_EXE)
    parser.add_argument('--self-test', action='store_true', help='run PNG decoder unit tests only')
    parser.add_argument('--check-existing', type=Path, help='verify existing screenshot directory')
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(DecoderTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if args.check_existing:
        results = Results()
        root = args.check_existing
        hashes = set()
        for body in BODIES:
            def check_existing_body(body=body):
                path = root / (body + '.png')
                hashes.add(image(path))
                width, height, _ = decode_png(path.read_bytes())
                require((width, height) == (1100, 760), 'unexpected dimensions for ' + body)
            results.check(body + ' existing PNG', check_existing_body)
        results.check('six distinct existing images', lambda: require(len(hashes) == 6, 'duplicate images'))
        def existing_gallery():
            path = root / 'gallery.png'
            width, height, _ = decode_png(path.read_bytes())
            require((width, height) == (3300, 1520), 'unexpected gallery dimensions')
            check_gallery(path)
        results.check('existing gallery full tiles', existing_gallery)
        print('RESULT: {} passed, {} failed, {} total'.format(results.passed, results.failed, results.passed + results.failed))
        return 1 if results.failed else 0
    exe = args.executable.resolve()
    if not exe.is_file():
        print('ERROR executable not found: {}'.format(exe), file=sys.stderr)
        return 1
    results = Results()
    # TMPDIR is supplied by Hermes; tempfile also has a safe platform fallback.
    with tempfile.TemporaryDirectory(prefix='lab5-verify-', dir=os.environ.get('TMPDIR') or None) as tmp:
        root = Path(tmp)
        results.check('render-all headless', lambda: succeeded(invoke(exe, root, '--headless', '--render-all')))
        hashes = {}
        for body in BODIES:
            def check_body(body=body):
                hashes[body] = image(root / 'screenshots' / (body + '.png'))
            results.check('{} PNG path, CRC, dimensions and diversity'.format(body), check_body)
        def distinct():
            require(len(hashes) == len(BODIES), 'not all images could be decoded')
            require(len(set(hashes.values())) == len(BODIES), 'two or more bodies have identical pixels')
        results.check('six distinct images', distinct)
        results.check('gallery has six distinct detailed panels',
                      lambda: check_gallery(root / 'screenshots' / 'gallery.png'))
        def exact_dimensions():
            for body in BODIES:
                width, height, _ = decode_png((root / 'screenshots' / (body + '.png')).read_bytes())
                require((width, height) == (1100, 760), 'unexpected dimensions for ' + body)
            width, height, _ = decode_png((root / 'screenshots' / 'gallery.png').read_bytes())
            require((width, height) == (3300, 1520), 'unexpected gallery dimensions')
        results.check('full-resolution image and gallery dimensions', exact_dimensions)
        rendered = {}
        for label, time in (('sol-t0-a', '0'), ('sol-t0-b', '0'), ('sol-t2', '2')):
            def check_render(label=label, time=time):
                path = root / (label + '.png')
                succeeded(invoke(exe, root, '--body', 'sol', '--headless', '--time', time, '--output', str(path)))
                rendered[label] = image(path)
            results.check(label + ' output', check_render)
        def compare(a, b, equal):
            require(a in rendered and b in rendered, 'required render failed')
            require((rendered[a] == rendered[b]) == equal, 'pixel hashes do not satisfy comparison')
        results.check('time 0 deterministic', lambda: compare('sol-t0-a', 'sol-t0-b', True))
        results.check('time 2 animation differs', lambda: compare('sol-t0-a', 'sol-t2', False))
        for label, flags in (
            ('unknown flag rejected', ('--headless', '--definitely-unknown-flag')),
            ('unknown body rejected', ('--headless', '--body', 'definitely-unknown-body')),
            ('malformed time rejected', ('--headless', '--time', 'not-a-number')),
            ('NaN time rejected', ('--headless', '--output', str(root / 'nan.png'), '--time', 'NaN')),
            ('infinite time rejected', ('--headless', '--output', str(root / 'inf.png'), '--time', 'inf')),
            ('missing output argument rejected', ('--headless', '--output')),
        ):
            def rejected(flags=flags):
                result = invoke(exe, root, *flags)
                require(result.returncode != 0, 'unexpected exit 0')
            results.check(label, rejected)
    print('RESULT: {} passed, {} failed, {} total'.format(results.passed, results.failed, results.passed + results.failed))
    return 1 if results.failed else 0


if __name__ == '__main__':
    sys.exit(main())
