use honggfuzz::fuzz;
use hexponent::{FloatLiteral, ConversionResult};

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            if let Ok(f) = FloatLiteral::from_bytes(data) {
                assert!(!f.clone().convert::<f32>().inner().is_nan());
                assert!(!f.convert::<f64>().inner().is_nan());
            }
        });
    }
}