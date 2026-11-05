#!/usr/bin/env python3
"""QA de imágenes y benchmark CPU; self-test no ejecuta render ni benchmarks."""
import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('verify_render', ROOT / 'tests' / 'verify_render.py')
if spec is None or spec.loader is None:
    raise ImportError('no se pudo cargar tests/verify_render.py')
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)
BODIES = verifier.BODIES
decode_png = verifier.decode_png

CASES = [(body, '0') for body in BODIES] + [('gaseoso', '2')]


def compare_pixels(reference, candidate):
    width, height, a = reference
    cw, ch, b = candidate
    if (width, height) != (cw, ch) or len(a) != width * height * 3 or len(b) != len(a):
        raise ValueError('dimensiones o longitud RGB distintas')
    pixels = channels = maximum = 0
    for offset in range(0, len(a), 3):
        changed = False
        for index in range(offset, offset + 3):
            delta = abs(a[index] - b[index])
            if delta:
                channels += 1
                changed = True
                maximum = max(maximum, delta)
        pixels += changed
    return {'different_pixels': pixels, 'different_channels': channels, 'max_delta': maximum}


def schedule(trials):
    return [('baseline', 'candidate') if trial % 2 == 0 else ('candidate', 'baseline')
            for trial in range(trials)]


def safe_output(path):
    path = path.resolve()
    if path == ROOT or ROOT in path.parents:
        raise ValueError('las salidas deben estar fuera del repositorio')
    return path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_benchmark(data, body, frames):
    results = data['results']
    if len(results) != 1 or results[0]['body'] != body:
        raise ValueError('cuerpo o cantidad de resultados inesperados')
    row = results[0]
    samples = row['samples']
    if len(samples) != frames or any(not isinstance(x, (int, float)) or not math.isfinite(x) or x <= 0 for x in samples):
        raise ValueError('muestras inválidas')
    sorted_samples = sorted(samples)
    expected = {'median_ms': statistics.median(samples),
                'p95_ms': sorted_samples[math.ceil(.95 * frames) - 1],
                'min_ms': min(samples), 'max_ms': max(samples)}
    expected['fps'] = 1000 / expected['median_ms']
    for key, value in expected.items():
        if not math.isfinite(row[key]) or not math.isclose(row[key], value, rel_tol=1e-8, abs_tol=1e-8):
            raise ValueError('estadística inválida: ' + key)
    return row


def write_json_new(path, value):
    with path.open('x', encoding='utf-8') as output:
        json.dump(value, output, indent=2, allow_nan=False)
        output.write('\n')


def capture(exe, directory):
    exe = exe.resolve(strict=True)
    directory = safe_output(directory)
    directory.mkdir(parents=True, exist_ok=False)
    for body, time in CASES:
        target = directory / f'{body}-t{time}.png'
        subprocess.run([str(exe), '--headless', '--body', body, '--time', time,
                        '--output', str(target)], check=True)
        decode_png(target.read_bytes())
    print('Capturas:', directory)


def compare(reference, candidate, output):
    output = safe_output(output)
    reference, candidate = reference.resolve(strict=True), candidate.resolve(strict=True)
    rows = []
    for body, time in CASES:
        name = f'{body}-t{time}.png'
        base, new = reference / name, candidate / name
        before, after = decode_png(base.read_bytes()), decode_png(new.read_bytes())
        rows.append({'file': name, 'width': before[0], 'height': before[1],
                     'reference_sha256': digest(base), 'candidate_sha256': digest(new),
                     **compare_pixels(before, after)})
    result = {'exact': all(r['different_channels'] == 0 for r in rows), 'images': rows}
    write_json_new(output, result)
    print(json.dumps(result, indent=2))
    return 0 if result['exact'] else 1


def run_benchmarks(baseline, candidate, directory, frames, warmup, time, trials):
    paths = {'baseline': baseline.resolve(strict=True), 'candidate': candidate.resolve(strict=True)}
    hashes = {key: digest(value) for key, value in paths.items()}
    if paths['baseline'] == paths['candidate']:
        raise ValueError('baseline y candidato deben ser archivos distintos')
    directory = safe_output(directory)
    directory.mkdir(parents=True, exist_ok=False)
    metadata = {'executables': {key: {'path': str(paths[key]), 'sha256': hashes[key]} for key in paths},
                'frames': frames, 'warmup': warmup, 'time': time, 'trials': trials,
                'order': schedule(trials), 'platform': platform.platform(),
                'processor': platform.processor(), 'python': platform.python_version()}
    write_json_new(directory / 'metadata.json', metadata)
    rows = []
    for trial, order in enumerate(schedule(trials), start=1):
        for body in BODIES:
            for version in order:
                command = [str(paths[version]), '--benchmark', '--frames', str(frames),
                           '--warmup', str(warmup), '--body', body, '--time', time]
                result = subprocess.run(command, check=True, capture_output=True, text=True)
                with (directory / f'trial{trial}-{body}-{version}.json').open('x', encoding='utf-8') as output:
                    output.write(result.stdout)
                row = validate_benchmark(json.loads(result.stdout), body, frames)
                rows.append({'trial': trial, 'version': version, **row})
    if any(digest(paths[key]) != hashes[key] for key in paths):
        raise ValueError('un ejecutable cambió durante la medición')
    summary = []
    for body in BODIES:
        versions = {version: [r for r in rows if r['body'] == body and r['version'] == version]
                    for version in ('baseline', 'candidate')}
        medians = {version: [r['median_ms'] for r in runs] for version, runs in versions.items()}
        base = statistics.median(medians['baseline'])
        new = statistics.median(medians['candidate'])
        summary.append({'body': body, 'baseline_median_of_medians_ms': base,
                        'candidate_median_of_medians_ms': new,
                        **{version + '_trial_medians_ms': values for version, values in medians.items()},
                        **{version + '_trial_medians_range_ms': [min(values), max(values)]
                           for version, values in medians.items()},
                        **{version + '_trial_p95_ms': [r['p95_ms'] for r in runs]
                           for version, runs in versions.items()},
                        **{version + '_sample_range_ms': [min(r['min_ms'] for r in runs),
                                                           max(r['max_ms'] for r in runs)]
                           for version, runs in versions.items()},
                        'speedup': base / new, 'baseline_fps': 1000 / base,
                        'candidate_fps': 1000 / new})
    write_json_new(directory / 'summary.json', {'results': summary})
    print(json.dumps({'results': summary}, indent=2))


class SelfTests(unittest.TestCase):
    def test_exact_pixels(self):
        self.assertEqual(compare_pixels((2, 1, bytes([1, 2, 3, 4, 5, 6])),
                                        (2, 1, bytes([1, 2, 3, 4, 5, 6]))),
                         {'different_pixels': 0, 'different_channels': 0, 'max_delta': 0})

    def test_channel_pixel_and_max_delta_counts(self):
        self.assertEqual(compare_pixels((2, 1, bytes([1, 2, 3, 4, 5, 6])),
                                        (2, 1, bytes([2, 4, 3, 4, 5, 0]))),
                         {'different_pixels': 2, 'different_channels': 3, 'max_delta': 6})

    def test_dimension_mismatch_is_an_error(self):
        with self.assertRaises(ValueError):
            compare_pixels((1, 1, b'\x00\x00\x00'), (2, 1, b'\x00' * 6))

    def test_alternates_trial_order(self):
        self.assertEqual(schedule(3), [('baseline', 'candidate'), ('candidate', 'baseline'),
                                       ('baseline', 'candidate')])

    def test_statistics_validation(self):
        row = {'body': 'sol', 'samples': [1, 2, 3], 'median_ms': 2, 'p95_ms': 3,
               'min_ms': 1, 'max_ms': 3, 'fps': 500}
        self.assertEqual(validate_benchmark({'results': [row]}, 'sol', 3), row)
        with self.assertRaises(ValueError):
            validate_benchmark({'results': [{**row, 'p95_ms': 2}]}, 'sol', 3)

    def test_repo_output_is_rejected(self):
        with self.assertRaises(ValueError):
            safe_output(ROOT / 'screenshots' / 'capture.png')

    def test_benchmark_protocol_with_simulated_subprocess(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('QA_SCRATCH')) as tmp:
            root = Path(tmp)
            baseline, candidate = root / 'baseline.exe', root / 'candidate.exe'
            baseline.write_bytes(b'baseline')
            candidate.write_bytes(b'candidate')
            calls = []

            def fake_run(command, **kwargs):
                calls.append(command)
                body = command[command.index('--body') + 1]
                return subprocess.CompletedProcess(command, 0, json.dumps({'results': [
                    {'body': body, 'samples': [2], 'median_ms': 2, 'p95_ms': 2,
                     'min_ms': 2, 'max_ms': 2, 'fps': 500}]}), '')

            with patch('subprocess.run', side_effect=fake_run), patch('builtins.print'):
                run_benchmarks(baseline, candidate, root / 'out', 1, 0, '0', 3)
            self.assertEqual(len(calls), 36)
            self.assertEqual([Path(calls[i][0]).name for i in (0, 1, 12, 13, 24, 25)],
                             ['baseline.exe', 'candidate.exe', 'candidate.exe',
                              'baseline.exe', 'baseline.exe', 'candidate.exe'])
            self.assertEqual(len(list((root / 'out').glob('trial*.json'))), 36)
            results = json.loads((root / 'out' / 'summary.json').read_text())['results']
            self.assertEqual(len(results), 6)
            self.assertEqual(results[0]['baseline_trial_medians_ms'], [2, 2, 2])
            self.assertEqual(results[0]['candidate_trial_medians_range_ms'], [2, 2])
            self.assertEqual(results[0]['baseline_sample_range_ms'], [2, 2])


def positive(value):
    number = int(value)
    if number <= 0:
        raise argparse.ArgumentTypeError('debe ser positivo')
    return number


def nonnegative(value):
    number = int(value)
    if number < 0:
        raise argparse.ArgumentTypeError('debe ser no negativo')
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    sub.add_parser('self-test')
    p = sub.add_parser('capture')
    p.add_argument('--exe', type=Path, required=True)
    p.add_argument('--output-dir', type=Path, required=True)
    p = sub.add_parser('compare')
    p.add_argument('--reference', type=Path, required=True)
    p.add_argument('--candidate', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p = sub.add_parser('run')
    p.add_argument('--baseline', type=Path, required=True)
    p.add_argument('--candidate', type=Path, required=True)
    p.add_argument('--output-dir', type=Path, required=True)
    p.add_argument('--frames', type=positive, default=12)
    p.add_argument('--warmup', type=nonnegative, default=2)
    p.add_argument('--time', default='0')
    p.add_argument('--trials', type=positive, default=3)
    args = parser.parse_args()
    if args.action == 'self-test':
        tests = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(tests).wasSuccessful() else 1
    try:
        if args.action == 'capture':
            capture(args.exe, args.output_dir)
        elif args.action == 'compare':
            return compare(args.reference, args.candidate, args.output)
        else:
            run_benchmarks(args.baseline, args.candidate, args.output_dir, args.frames,
                           args.warmup, args.time, args.trials)
        return 0
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        print('ERROR:', exc, file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
