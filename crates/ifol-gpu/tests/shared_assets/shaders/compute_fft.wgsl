@group(0) @binding(0) var<storage, read> waveform: array<f32>;
@group(0) @binding(1) var<storage, read_write> spectrum: array<f32>;

var<workgroup> shared_data: array<vec2<f32>, 256>;

const PI: f32 = 3.14159265359;

fn reverseBits8(x: u32) -> u32 {
    var v = x;
    v = ((v >> 1u) & 0x55u) | ((v & 0x55u) << 1u);
    v = ((v >> 2u) & 0x33u) | ((v & 0x33u) << 2u);
    v = ((v >> 4u) & 0x0Fu) | ((v & 0x0Fu) << 4u);
    return v;
}

@compute @workgroup_size(256)
fn cs_main(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let tid = local_id.x;
    
    let reversed = reverseBits8(tid);
    
    // Hann window
    let window = 0.5 * (1.0 - cos(2.0 * PI * f32(tid) / 255.0));
    shared_data[reversed] = vec2<f32>(waveform[tid] * window, 0.0);
    workgroupBarrier();
    
    for (var s = 1u; s <= 8u; s++) {
        let m = 1u << s;
        let half_m = m >> 1u;
        
        let k = tid & (m - 1u);
        let pos_even = (tid & ~(m - 1u)) + k;
        let pos_odd = pos_even + half_m;
        
        var next_even: vec2<f32>;
        var next_odd: vec2<f32>;
        var do_write = false;

        if (k < half_m) {
            let angle = -2.0 * PI * f32(k) / f32(m);
            let twiddle = vec2<f32>(cos(angle), sin(angle));
            
            let even = shared_data[pos_even];
            let odd = shared_data[pos_odd];
            
            let t = vec2<f32>(
                twiddle.x * odd.x - twiddle.y * odd.y,
                twiddle.x * odd.y + twiddle.y * odd.x
            );
            
            next_even = even + t;
            next_odd = even - t;
            do_write = true;
        }
        
        workgroupBarrier();
        
        if (do_write) {
            shared_data[pos_even] = next_even;
            shared_data[pos_odd]  = next_odd;
        }
        
        workgroupBarrier();
    }
    
    if (tid < 128u) {
        let val = shared_data[tid];
        let mag = sqrt(val.x * val.x + val.y * val.y) / 128.0;
        
        // Log scale (dB) normalized to [0, 1]
        var db = 20.0 * log2(mag + 1e-6);
        if (db < -60.0) { db = -60.0; }
        let norm_db = (db + 60.0) / 60.0;
        spectrum[tid] = norm_db;
    }
}
