#!/usr/bin/env python3
"""Headless checks for control CLI and pixel-preserving defaults; never writes repository captures."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from verify_render import BODIES, decode_png

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / 'target' / 'release' / ('lab5-graficas.exe' if os.name == 'nt' else 'lab5-graficas')
FIXTURES = None


def same_pixels(a, b):
    return a == b


def assert_changed(test, base, altered):
    test.assertEqual(base[:2], altered[:2], 'image dimensions changed')
    test.assertNotEqual(base[2], altered[2], 'expected RGB pixels to change')


class HarnessTests(unittest.TestCase):
    def test_default_mode_without_external_fixtures(self):
        subject = ControlsTests('test_defaults_match_original_seven_fixtures')
        image = (1, 1, b'\x01\x02\x03')
        with patch.dict(subject.__dict__, {'render': lambda *args, **kwargs: image}), \
                patch.dict(globals(), {'FIXTURES': None, 'decode_png': lambda data: image}), \
                patch.object(Path, 'read_bytes', return_value=b'png'):
            subject.test_defaults_match_original_seven_fixtures()

    def test_strict_mode_rejects_missing_fixture(self):
        subject = ControlsTests('test_defaults_match_original_seven_fixtures')
        image = (1, 1, b'\x01\x02\x03')
        with patch.dict(subject.__dict__, {'render': lambda *args, **kwargs: image}), \
                patch.dict(globals(), {'FIXTURES': Path('missing'), 'decode_png': lambda data: image}), \
                patch.object(Path, 'read_bytes', return_value=b'png'), \
                patch.object(Path, 'is_file', return_value=False):
            with self.assertRaises(AssertionError):
                subject.test_defaults_match_original_seven_fixtures()

    def test_exact_comparison_detects_one_channel(self):
        a = (1, 1, b'\x01\x02\x03')
        self.assertTrue(same_pixels(a, a))
        self.assertFalse(same_pixels(a, (1, 1, b'\x01\x02\x04')))

    def test_changed_requires_dimensions_and_pixels(self):
        a = (1, 1, b'\x01\x02\x03')
        assert_changed(self, a, (1, 1, b'\x01\x02\x04'))
        with self.assertRaises(AssertionError):
            assert_changed(self, a, a)
        with self.assertRaises(AssertionError):
            assert_changed(self, a, (2, 1, b'\x01\x02\x04' * 2))


class ControlsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.exe = EXE.resolve(strict=True)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='lab5-controls-', dir=os.environ.get('QA_SCRATCH') or None)
        self.root = Path(self.temp.name).resolve()

    def tearDown(self):
        self.temp.cleanup()

    def invoke(self, *args):
        return subprocess.run([str(self.exe), *map(str, args)], cwd=self.root,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              timeout=90, check=False)

    def render(self, name, body='gaseoso', time='0', flags=(), headless=True):
        target = self.root / name
        args = (['--headless'] if headless else []) + ['--body', body, '--time', time,
                                                        '--output', str(target), *flags]
        result = self.invoke(*args)
        self.assertEqual(result.returncode, 0, result.stderr.decode('utf-8', 'replace'))
        self.assertTrue(target.is_file(), 'missing output PNG')
        image = decode_png(target.read_bytes())
        self.assertEqual(image[:2], (1100, 760))
        return image

    def test_help_exits_without_window(self):
        result = self.invoke('--help')
        self.assertEqual(result.returncode, 0, result.stderr.decode('utf-8', 'replace'))
        text = result.stdout.decode('utf-8', 'replace')
        for flag in ('--body', '--brightness', '--detail', '--help-overlay', '--headless',
                     '--output', '--time', '--render-all', '--benchmark', '--frames', '--warmup'):
            self.assertIn(flag, text)
        self.assertEqual(list(self.root.iterdir()), [], 'help should not create files')

    def test_invalid_brightness_and_detail_rejected(self):
        for flag, values in (('--brightness', ('NaN', 'inf', '-inf', '0.24', '2.01', 'bad')),
                             ('--detail', ('NaN', 'inf', '-inf', '0.49', '2.01', 'bad'))):
            for value in values:
                with self.subTest(flag=flag, value=value):
                    output = self.root / 'invalid.png'
                    result = self.invoke('--headless', '--output', output, flag, value)
                    self.assertNotEqual(result.returncode, 0, f'{flag} {value} accepted')
                    self.assertFalse(output.exists(), 'invalid arguments wrote PNG')

    def test_range_endpoints_accepted(self):
        for flag, values in (('--brightness', ('0.25', '2')), ('--detail', ('0.5', '2'))):
            for index, value in enumerate(values):
                with self.subTest(flag=flag, value=value):
                    self.render(f'{flag[2:]}-{index}.png', flags=(flag, value))

    def test_defaults_match_original_seven_fixtures(self):
        fixtures = FIXTURES
        for body, time in [(body, '0') for body in BODIES] + [('gaseoso', '2')]:
            with self.subTest(body=body, time=time):
                implicit = self.render(f'{body}-{time}-implicit.png', body, time)
                explicit = self.render(f'{body}-{time}-explicit.png', body, time,
                                       flags=('--brightness', '1', '--detail', '1'))
                self.assertTrue(same_pixels(implicit, explicit), 'implicit and explicit defaults differ')
                if time == '0':
                    committed = decode_png((ROOT / 'screenshots' / f'{body}.png').read_bytes())
                    self.assertTrue(same_pixels(committed, implicit), 'default differs from repository screenshot')
                if fixtures is not None:
                    fixture = fixtures / f'{body}-t{time}.png'
                    self.assertTrue(fixture.is_file(), f'missing original fixture: {fixture}')
                    original = decode_png(fixture.read_bytes())
                    self.assertTrue(same_pixels(original, implicit), 'default differs from original fixture')

    def test_brightness_changes_pixels(self):
        base = self.render('brightness-base.png')
        changed = self.render('brightness-adjusted.png', flags=('--brightness', '1.5'))
        assert_changed(self, base, changed)

    def test_detail_changes_pixels(self):
        base = self.render('detail-base.png')
        changed = self.render('detail-adjusted.png', flags=('--detail', '1.5'))
        assert_changed(self, base, changed)

    def test_overlay_is_individual_export_opt_in(self):
        plain = self.render('clean.png')
        direct = self.render('clean-direct.png', headless=False)
        overlay = self.render('overlay.png', flags=('--help-overlay',))
        self.assertTrue(same_pixels(plain, direct), 'default export unexpectedly differs')
        assert_changed(self, plain, overlay)
        for args in (('--headless', '--help-overlay'),
                     ('--render-all', '--headless', '--help-overlay', '--output', self.root / 'gallery')):
            with self.subTest(args=args):
                result = self.invoke(*args)
                self.assertNotEqual(result.returncode, 0, 'overlay accepted outside individual export')


def main():
    global EXE, FIXTURES
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', nargs='?', type=Path, default=EXE)
    parser.add_argument('--fixtures', type=Path, help='original seven PNGs named <body>-t<time>.png')
    parser.add_argument('--self-test', action='store_true', help='test verifier logic only; no renderer')
    parser.add_argument('--case', help='run one ControlsTests method (for focused diagnosis)')
    args = parser.parse_args()
    EXE = args.executable
    FIXTURES = args.fixtures.resolve() if args.fixtures else None
    if not args.self_test:
        if FIXTURES is None:
            print('Reference mode: six repository screenshots at t=0; gaseoso t=2 compares implicit vs explicit defaults only (no original fixture).', flush=True)
        else:
            print(f'Strict reference mode: seven original fixtures in {FIXTURES}, plus six repository screenshots.', flush=True)
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(HarnessTests)
    elif args.case:
        if not args.case.startswith('test_') or not callable(getattr(ControlsTests, args.case, None)):
            parser.error('unknown ControlsTests method')
        suite = unittest.TestSuite([ControlsTests(args.case)])
    else:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ControlsTests)
    return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1


if __name__ == '__main__':
    sys.exit(main())
