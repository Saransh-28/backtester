// prepare_inputs.rs

pub fn prepare_inputs(arrays: &mut [&mut Vec<f64>]) -> Result<usize, &'static str> {
    let len = arrays[0].len();
    for arr in arrays.iter() {
        if arr.len() != len {
            return Err("All input arrays must have the same length");
        }
        if arr.iter().any(|x| x.is_nan()) {
            return Err("Input contains NaN");
        }
    }
    Ok(len)
}
