const MARGIN: usize = 4;
const ALPHA: f64 = -1.586_134_342_059_924;
const BETA: f64 = -0.052_980_118_572_961;
const GAMMA: f64 = 0.882_911_075_530_934;
const DELTA: f64 = 0.443_506_852_043_971;
const K: f64 = 1.230_174_104_914_001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wavelet {
    Reversible,
    Irreversible,
}

impl Wavelet {
    pub fn synthesize(self, line: &mut [f64], start: usize) {
        let count = line.len();
        if count == 0 {
            return;
        }
        if count == 1 {
            if start % 2 == 1 {
                line[0] = match self {
                    Wavelet::Reversible => (line[0] as i64 / 2) as f64,
                    Wavelet::Irreversible => line[0] / 2.0,
                };
            }
            return;
        }
        let mut extended = vec![0.0; count + 2 * MARGIN];
        for (index, slot) in extended.iter_mut().enumerate() {
            *slot = line[reflect(index as i64 - MARGIN as i64, count)];
        }
        let parity = (start + MARGIN) % 2;
        let mut lift = |odd: bool, step: &dyn Fn(f64, f64, f64) -> f64| {
            let first = if (parity == 1) != odd { 1 } else { 2 };
            let mut index = first;
            while index + 1 < extended.len() {
                extended[index] = step(extended[index], extended[index - 1], extended[index + 1]);
                index += 2;
            }
        };
        match self {
            Wavelet::Reversible => {
                lift(false, &|value, before, after| value - ((before + after + 2.0) / 4.0).floor());
                lift(true, &|value, before, after| value + ((before + after) / 2.0).floor());
            }
            Wavelet::Irreversible => {
                lift(false, &|value, _, _| value * K);
                lift(true, &|value, _, _| value / K);
                lift(false, &|value, before, after| value - DELTA * (before + after));
                lift(true, &|value, before, after| value - GAMMA * (before + after));
                lift(false, &|value, before, after| value - BETA * (before + after));
                lift(true, &|value, before, after| value - ALPHA * (before + after));
            }
        }
        line.copy_from_slice(&extended[MARGIN..MARGIN + count]);
    }
}

fn reflect(index: i64, count: usize) -> usize {
    let period = 2 * (count as i64 - 1);
    let folded = index.rem_euclid(period);
    (if folded >= count as i64 { period - folded } else { folded }) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reversible_filter_undoes_its_forward_lifting() {
        let signal: Vec<f64> = vec![12.0, 200.0, 37.0, 37.0, 90.0, 4.0, 255.0];
        for start in [0, 1] {
            let count = signal.len();
            let at = |values: &[f64], index: i64| values[reflect(index, count)];
            let mut forward = signal.clone();
            for index in 0..count {
                if (start + index) % 2 == 1 {
                    forward[index] =
                        signal[index] - ((at(&signal, index as i64 - 1) + at(&signal, index as i64 + 1)) / 2.0).floor();
                }
            }
            let high = forward.clone();
            for index in 0..count {
                if (start + index) % 2 == 0 {
                    forward[index] = signal[index]
                        + ((at(&high, index as i64 - 1) + at(&high, index as i64 + 1) + 2.0) / 4.0).floor();
                }
            }
            Wavelet::Reversible.synthesize(&mut forward, start);
            assert_eq!(forward, signal, "start {start}");
        }
    }
}
