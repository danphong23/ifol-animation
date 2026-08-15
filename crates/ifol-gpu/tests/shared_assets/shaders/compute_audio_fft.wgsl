// 64 Frequency Bins Compute FFT Shader

@group(0) @binding(0) var<storage, read> audio_samples: array<f32>; // 4096 samples
@group(0) @binding(1) var<storage, read_write> spectrum_energy: array<f32, 64>; // 64 bins

struct AudioParams {
    sample_count: u32,
    smoothing: f32,
    gain: f32,
    pad: f32,
};

@group(0) @binding(2) var<uniform> params: AudioParams;

@compute @workgroup_size(64, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let bin_idx = global_id.x;
    if (bin_idx >= 64u) {
        return;
    }

    let n = params.sample_count;
    
    // Logarithmic frequency distribution (20 Hz -> 20,000 Hz)
    let min_freq = 2.0;
    let max_freq = f32(n) / 4.0;
    let freq = min_freq * pow(max_freq / min_freq, f32(bin_idx) / 63.0);

    var real_sum = 0.0;
    var imag_sum = 0.0;

    let pi2 = 6.28318530718;

    // Windowed Discrete Fourier transform for this frequency bin
    for (var i = 0u; i < n; i = i + 1u) {
        let sample = audio_samples[i];
        
        // Hann window function to eliminate spectral leakage
        let t = f32(i) / f32(n);
        let window = 0.5 * (1.0 - cos(pi2 * t));
        let windowed_sample = sample * window;

        let angle = pi2 * freq * t;
        real_sum = real_sum + windowed_sample * cos(angle);
        imag_sum = imag_sum - windowed_sample * sin(angle);
    }

    // Magnitude calculation: sqrt(real^2 + imag^2) / N
    let magnitude = sqrt(real_sum * real_sum + imag_sum * imag_sum) / (f32(n) * 0.25);
    
    // Decibel logarithmic scale normalization [0.0 .. 1.0]
    let db_val = clamp(log(magnitude * 100.0 + 1.0) / log(101.0) * params.gain, 0.0, 1.0);

    spectrum_energy[bin_idx] = db_val;
}
