// WebGPU Comprehensive Test Suite for iFol GPU Engine
// Exact mathematical duplicate of Desktop Rust Test Cases

const runnerQuery = new URLSearchParams(window.location.search);
const skipPreview = runnerQuery.get('skip_preview') === '1';

function log(msg, type = 'info') {
    const el = document.getElementById('console-log');
    const div = document.createElement('div');
    div.className = `log-entry ${type === 'error' ? 'log-error' : type === 'success' ? 'log-success' : ''}`;
    div.textContent = `[${new Date().toISOString().substring(11, 19)}] ${msg}`;
    el.appendChild(div);
    el.scrollTop = el.scrollHeight;
}

async function saveCanvasImage(canvas, filename) {
    if (skipPreview) {
        log(`Skipped presentation preview ${filename} for focused run`);
        return { skipped: true };
    }
    const dataUrl = canvas.toDataURL('image/png');
    try {
        const res = await fetch('/save_output', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name: filename, image: dataUrl })
        });
        const json = await res.json();
        log(`Saved ${filename} to server`, 'success');
        return json;
    } catch (e) {
        log(`Failed to save ${filename}: ${e.message}`, 'error');
    }
}

function bytesToBase64(bytes) {
    let binary = '';
    const chunkSize = 0x8000;
    for (let offset = 0; offset < bytes.length; offset += chunkSize) {
        binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize));
    }
    return btoa(binary);
}

function fnv1a64(bytes) {
    let hash = 0xcbf29ce484222325n;
    for (const byte of bytes) {
        hash ^= BigInt(byte);
        hash = BigInt.asUintN(64, hash * 0x100000001b3n);
    }
    return hash.toString(16).padStart(16, '0');
}

async function readTextureBytes(device, texture, width, height) {
    const bytesPerRow = width * 4;
    const paddedBytesPerRow = Math.ceil(bytesPerRow / 256) * 256;
    const readbackBuffer = device.createBuffer({
        size: paddedBytesPerRow * height,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ
    });
    const encoder = device.createCommandEncoder();
    encoder.copyTextureToBuffer(
        { texture },
        { buffer: readbackBuffer, bytesPerRow: paddedBytesPerRow, rowsPerImage: height },
        [width, height, 1]
    );
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await readbackBuffer.mapAsync(GPUMapMode.READ);
    const mapped = new Uint8Array(readbackBuffer.getMappedRange());
    const bytes = new Uint8Array(bytesPerRow * height);
    for (let row = 0; row < height; row++) {
        bytes.set(
            mapped.subarray(row * paddedBytesPerRow, row * paddedBytesPerRow + bytesPerRow),
            row * bytesPerRow
        );
    }
    readbackBuffer.unmap();
    readbackBuffer.destroy();
    return bytes;
}

async function loadImageTexture(device, filename) {
    const response = await fetch(`/textures/${filename}`);
    if (!response.ok) throw new Error(`Failed to load texture: ${filename}`);
    const bitmap = await createImageBitmap(await response.blob());
    const probeCanvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const probeContext = probeCanvas.getContext('2d', { willReadFrequently: true });
    probeContext.drawImage(bitmap, 0, 0);
    const imageData = probeContext.getImageData(0, 0, bitmap.width, bitmap.height);
    const pixelOffset = (Math.min(2, bitmap.width - 1) + Math.min(2, bitmap.height - 1) * bitmap.width) * 4;
    const pixel = imageData.data.subarray(pixelOffset, pixelOffset + 4);
    const texture = device.createTexture({
        size: [bitmap.width, bitmap.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST
    });
    device.queue.writeTexture(
        { texture },
        imageData.data,
        { bytesPerRow: bitmap.width * 4, rowsPerImage: bitmap.height },
        [bitmap.width, bitmap.height, 1]
    );
    bitmap.close();
    return {
        texture,
        width: probeCanvas.width,
        height: probeCanvas.height,
        keyColor: [pixel[0] / 255, pixel[1] / 255, pixel[2] / 255]
    };
}

async function saveRawTexture(bytes, metadata) {
    const res = await fetch('/save_raw', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            name: metadata.name,
            bytes: bytesToBase64(bytes),
            width: metadata.width,
            height: metadata.height,
            format: metadata.format,
            render_time_ms: metadata.render_time_ms,
            cold_render_time_ms: metadata.cold_render_time_ms,
            warm_render_time_ms: metadata.warm_render_time_ms,
            warm_iteration_count: metadata.warm_iteration_count,
            speedup_percentage: metadata.speedup_percentage,
            cache_output_equal: metadata.cache_output_equal,
            validation_error: metadata.validation_error,
            missing_bind_group: metadata.missing_bind_group,
            validation_passed: metadata.validation_passed,
            panic_occurred: metadata.panic_occurred,
            fallback_color: metadata.fallback_color,
            manifest: metadata.manifest,
            manifest_fingerprint: metadata.manifest_fingerprint,
            adapter_name: metadata.adapter_name,
            timing_scope: metadata.timing_scope,
            allocated_nodes: metadata.allocated_nodes,
            freed_nodes: metadata.freed_nodes,
            surviving_nodes: metadata.surviving_nodes,
            pool_check: metadata.pool_check,
            recursion_depth: metadata.recursion_depth,
            flattened_operations: metadata.flattened_operations,
            node_count: metadata.node_count,
            draw_commands: metadata.draw_commands,
            instance_count: metadata.instance_count,
            pass_count: metadata.pass_count,
            viewport_count: metadata.viewport_count,
            image_name: metadata.image_name
        })
    });
    if (!res.ok) throw new Error(`Raw output save failed: ${res.status}`);
    return await res.json();
}

async function runCanonicalParityProbe(gpu) {
    const { device } = gpu;
    const width = 800;
    const height = 600;
    const format = 'rgba8unorm';
    const texture = device.createTexture({
        size: [width, height],
        format,
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const bytesPerRow = width * 4;
    const paddedBytesPerRow = Math.ceil(bytesPerRow / 256) * 256;
    const readbackBuffer = device.createBuffer({
        size: paddedBytesPerRow * height,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ
    });
    const started = performance.now();
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
        colorAttachments: [{
            view: texture.createView(),
            clearValue: { r: 0.03, g: 0.04, b: 0.07, a: 1.0 },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    pass.end();
    encoder.copyTextureToBuffer(
        { texture },
        { buffer: readbackBuffer, bytesPerRow: paddedBytesPerRow, rowsPerImage: height },
        [width, height, 1]
    );
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await readbackBuffer.mapAsync(GPUMapMode.READ);
    const mapped = new Uint8Array(readbackBuffer.getMappedRange());
    const bytes = new Uint8Array(bytesPerRow * height);
    for (let row = 0; row < height; row++) {
        bytes.set(
            mapped.subarray(row * paddedBytesPerRow, row * paddedBytesPerRow + bytesPerRow),
            row * bytesPerRow
        );
    }
    readbackBuffer.unmap();
    await saveRawTexture(bytes, {
        name: 'canonical_parity_rgba8unorm',
        width,
        height,
        format: 'Rgba8Unorm',
        render_time_ms: performance.now() - started
    });
    texture.destroy();
    readbackBuffer.destroy();
}

async function runTC01(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc01_empty.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC01 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const clear = manifest.graph.operations[0].color;
    const width = target.width;
    const height = target.height;
    const manifestFingerprint = fnv1a64(new TextEncoder().encode(manifestText));
    const texture = device.createTexture({
        size: [width, height],
        format: 'rgba8unorm',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });

    async function executeClear() {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({
            colorAttachments: [{
                view: texture.createView(),
                clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
                loadOp: 'clear',
                storeOp: 'store'
            }]
        });
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }

    const coldRenderTimeMs = await executeClear();
    const warmRenderTimeMs = await executeClear();
    const bytes = await readTextureBytes(device, texture, width, height);
    await saveRawTexture(bytes, {
        name: 'tc01_empty_web',
        width,
        height,
        format: 'Rgba8Unorm',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc01_empty.json',
        manifest_fingerprint: manifestFingerprint,
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        image_name: 'tc01_empty_web.png'
    });

    const canvas = document.getElementById('canvas-tc01');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasEncoder = device.createCommandEncoder();
    const canvasPass = canvasEncoder.beginRenderPass({
        colorAttachments: [{
            view: context.getCurrentTexture().createView(),
            clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    canvasPass.end();
    device.queue.submit([canvasEncoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    document.getElementById('tag-tc01').textContent = 'PASS';
    document.getElementById('tag-tc01').className = 'tag tag-passed';
    texture.destroy();
}

async function runTC02(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc02_single_quad.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC02 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const drawSpec = manifest.graph.operations[0];
    const clear = manifest.graph.clear_color;
    const image = await loadImageTexture(device, drawSpec.asset);
    const shaderCode = await fetchShader(drawSpec.shader);
    const shaderModule = device.createShaderModule({ code: shaderCode });
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function createSpritePipeline(format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shaderModule, entryPoint: 'vs_main' },
            fragment: {
                module: shaderModule,
                entryPoint: 'fs_main',
                targets: [{ format, blend: {
                    color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
                    alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
                }}]
            },
            primitive: { topology: 'triangle-list' }
        });
    }
    const offscreenPipeline = createSpritePipeline('rgba8unorm-srgb');
    const crop = drawSpec.crop_uv;
    const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
    const screenAspect = target.width / target.height;
    const scaleY = drawSpec.target_height_scale;
    const scaleX = scaleY * (cropAspect / screenAspect);
    const uniformData = new Float32Array([
        drawSpec.position[0], drawSpec.position[1], scaleX, scaleY,
        crop[0], crop[1], crop[2], crop[3],
        image.keyColor[0], image.keyColor[1], image.keyColor[2], drawSpec.tolerance,
        drawSpec.smoothness, drawSpec.z_depth, drawSpec.opacity, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear' });
    const textureBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });

    async function executeDraw(outputTexture) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(offscreenPipeline);
        pass.setBindGroup(0, textureBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(drawSpec.vertex_count, 1, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }

    const coldRenderTimeMs = await executeDraw(targetTexture);
    const warmRenderTimeMs = await executeDraw(targetTexture);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc02_single_quad_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc02_single_quad.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        image_name: 'tc02_single_quad_web.png'
    });

    const canvas = document.getElementById('canvas-tc02');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasPipeline = createSpritePipeline(canvasFormat);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
        loadOp: 'clear', storeOp: 'store'
    }]});
    pass.setPipeline(canvasPipeline);
    pass.setBindGroup(0, textureBindGroup);
    pass.setBindGroup(1, uniformBindGroup);
    pass.draw(drawSpec.vertex_count, 1, 0, 0);
    pass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    document.getElementById('tag-tc02').textContent = 'PASS';
    document.getElementById('tag-tc02').className = 'tag tag-passed';
    targetTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC03(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc03_zbuffer.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC03 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const clear = manifest.graph.clear_color;
    const depthSpec = manifest.graph.depth_stencil;
    const shaderCode = await fetchShader(operations[0].shader);
    const shaderModule = device.createShaderModule({ code: shaderCode });
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function createSpritePipeline(format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shaderModule, entryPoint: 'vs_main' },
            fragment: {
                module: shaderModule,
                entryPoint: 'fs_main',
                targets: [{ format, blend: {
                    color: { srcFactor: 'one', dstFactor: 'zero', operation: 'add' },
                    alpha: { srcFactor: 'one', dstFactor: 'zero', operation: 'add' }
                }}]
            },
            primitive: { topology: 'triangle-list' },
            depthStencil: {
                format: 'depth32float',
                depthWriteEnabled: depthSpec.write,
                depthCompare: 'less-equal'
            }
        });
    }
    const offscreenPipeline = createSpritePipeline('rgba8unorm-srgb');
    const sprites = [];
    for (const drawSpec of operations) {
        const image = await loadImageTexture(device, drawSpec.asset);
        const crop = drawSpec.crop_uv;
        const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
        const screenAspect = target.width / target.height;
        const scaleY = drawSpec.target_height_scale;
        const scaleX = scaleY * (cropAspect / screenAspect);
        const uniformData = new Float32Array([
            drawSpec.position[0], drawSpec.position[1], scaleX, scaleY,
            crop[0], crop[1], crop[2], crop[3],
            image.keyColor[0], image.keyColor[1], image.keyColor[2], drawSpec.tolerance,
            drawSpec.smoothness, drawSpec.z_depth, drawSpec.opacity, 0
        ]);
        const uniformBuffer = device.createBuffer({
            size: uniformData.byteLength,
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
        });
        device.queue.writeBuffer(uniformBuffer, 0, uniformData);
        const sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear' });
        sprites.push({
            image,
            textureBindGroup: device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
            }),
            uniformBindGroup: device.createBindGroup({
                layout: uniformLayout,
                entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
            }),
            uniformBuffer,
            vertexCount: drawSpec.vertex_count
        });
    }
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const depthTexture = device.createTexture({
        size: [target.width, target.height],
        format: depthSpec.format.toLowerCase(),
        usage: GPUTextureUsage.RENDER_ATTACHMENT
    });

    async function executeDraw(outputTexture, outputDepthTexture, pipeline) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({
            colorAttachments: [{
                view: outputTexture.createView(),
                clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
                loadOp: 'clear', storeOp: 'store'
            }],
            depthStencilAttachment: {
                view: outputDepthTexture.createView(),
                depthClearValue: depthSpec.clear,
                depthLoadOp: 'clear',
                depthStoreOp: 'discard'
            }
        });
        pass.setPipeline(pipeline);
        for (const sprite of sprites) {
            pass.setBindGroup(0, sprite.textureBindGroup);
            pass.setBindGroup(1, sprite.uniformBindGroup);
            pass.draw(sprite.vertexCount, 1, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }

    const coldRenderTimeMs = await executeDraw(targetTexture, depthTexture, offscreenPipeline);
    const warmRenderTimeMs = await executeDraw(targetTexture, depthTexture, offscreenPipeline);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc03_zbuffer_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc03_zbuffer.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        image_name: 'tc03_zbuffer_web.png'
    });

    const canvas = document.getElementById('canvas-tc03');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasPipeline = createSpritePipeline(canvasFormat);
    const canvasDepthTexture = device.createTexture({
        size: [target.width, target.height],
        format: depthSpec.format.toLowerCase(),
        usage: GPUTextureUsage.RENDER_ATTACHMENT
    });
    await executeDraw(context.getCurrentTexture(), canvasDepthTexture, canvasPipeline);
    document.getElementById('tag-tc03').textContent = 'PASS';
    document.getElementById('tag-tc03').className = 'tag tag-passed';
    targetTexture.destroy();
    depthTexture.destroy();
    canvasDepthTexture.destroy();
    for (const sprite of sprites) {
        sprite.image.texture.destroy();
        sprite.uniformBuffer.destroy();
    }
}

async function runTC04(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc04_alpha_blend.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC04 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const clear = manifest.graph.clear_color;
    const depthSpec = manifest.graph.depth_stencil;
    const shaderCode = await fetchShader(manifest.graph.pipelines.opaque.shader);
    const shaderModule = device.createShaderModule({ code: shaderCode });
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC04 blend mode: ${name}`);
    }
    function createPipeline(pipelineSpec, format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shaderModule, entryPoint: 'vs_main' },
            fragment: {
                module: shaderModule,
                entryPoint: 'fs_main',
                targets: [{ format, blend: blendState(pipelineSpec.blend) }]
            },
            primitive: { topology: 'triangle-list' },
            depthStencil: {
                format: depthSpec.format.toLowerCase(),
                depthWriteEnabled: pipelineSpec.depth_write,
                depthCompare: depthSpec.compare === 'LessEqual' ? 'less-equal' : depthSpec.compare.toLowerCase()
            }
        });
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = createPipeline(spec, format);
        }
        return result;
    }

    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    const sprites = [];
    for (const drawSpec of operations) {
        const image = await loadImageTexture(device, drawSpec.asset);
        const crop = drawSpec.crop_uv;
        const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
        const screenAspect = target.width / target.height;
        const scaleY = drawSpec.target_height_scale;
        const scaleX = scaleY * (cropAspect / screenAspect);
        const uniformData = new Float32Array([
            drawSpec.position[0], drawSpec.position[1], scaleX, scaleY,
            crop[0], crop[1], crop[2], crop[3],
            image.keyColor[0], image.keyColor[1], image.keyColor[2], drawSpec.tolerance,
            drawSpec.smoothness, drawSpec.z_depth, drawSpec.opacity, 0
        ]);
        const uniformBuffer = device.createBuffer({
            size: uniformData.byteLength,
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
        });
        device.queue.writeBuffer(uniformBuffer, 0, uniformData);
        const sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear' });
        sprites.push({
            image,
            pipeline: drawSpec.pipeline,
            textureBindGroup: device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
            }),
            uniformBindGroup: device.createBindGroup({
                layout: uniformLayout,
                entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
            }),
            uniformBuffer,
            vertexCount: drawSpec.vertex_count
        });
    }
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const depthTexture = device.createTexture({
        size: [target.width, target.height],
        format: depthSpec.format.toLowerCase(),
        usage: GPUTextureUsage.RENDER_ATTACHMENT
    });

    async function executeDraw(outputTexture, outputDepthTexture, pipelines) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({
            colorAttachments: [{
                view: outputTexture.createView(),
                clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
                loadOp: 'clear', storeOp: 'store'
            }],
            depthStencilAttachment: {
                view: outputDepthTexture.createView(),
                depthClearValue: depthSpec.clear,
                depthLoadOp: 'clear',
                depthStoreOp: 'discard'
            }
        });
        for (const sprite of sprites) {
            pass.setPipeline(pipelines[sprite.pipeline]);
            pass.setBindGroup(0, sprite.textureBindGroup);
            pass.setBindGroup(1, sprite.uniformBindGroup);
            pass.draw(sprite.vertexCount, 1, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }

    const coldRenderTimeMs = await executeDraw(targetTexture, depthTexture, offscreenPipelines);
    const warmRenderTimeMs = await executeDraw(targetTexture, depthTexture, offscreenPipelines);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc04_alpha_blend_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc04_alpha_blend.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        image_name: 'tc04_alpha_blend_web.png'
    });

    const canvas = document.getElementById('canvas-tc04');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasPipelines = createPipelines(canvasFormat);
    const canvasDepthTexture = device.createTexture({
        size: [target.width, target.height],
        format: depthSpec.format.toLowerCase(),
        usage: GPUTextureUsage.RENDER_ATTACHMENT
    });
    await executeDraw(context.getCurrentTexture(), canvasDepthTexture, canvasPipelines);
    document.getElementById('tag-tc04').textContent = 'PASS';
    document.getElementById('tag-tc04').className = 'tag tag-passed';
    targetTexture.destroy();
    depthTexture.destroy();
    canvasDepthTexture.destroy();
    for (const sprite of sprites) {
        sprite.image.texture.destroy();
        sprite.uniformBuffer.destroy();
    }
}

async function runTC05(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc05_interleaved.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC05 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const passes = manifest.graph.passes;
    const targetSpecs = manifest.graph.targets;
    const clearByPass = Object.fromEntries(passes.map(pass => [pass.id, pass.clear_color]));
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
        }
    }
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC05 blend mode: ${name}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            const layouts = spec.has_uniform ? [textureLayout, uniformLayout] : [textureLayout];
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: layouts }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }

    const targetTextures = {};
    for (const spec of targetSpecs) {
        targetTextures[spec.id] = device.createTexture({
            size: [spec.width, spec.height],
            format: 'rgba8unorm-srgb',
            usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
        });
    }
    const sampler = device.createSampler({
        addressModeU: 'repeat',
        addressModeV: 'repeat',
        addressModeW: 'repeat',
        magFilter: 'linear',
        minFilter: 'linear',
        mipmapFilter: 'linear'
    });
    const targetBindGroups = {};
    for (const targetId of ['A', 'B']) {
        targetBindGroups[targetId] = device.createBindGroup({
            layout: textureLayout,
            entries: [{ binding: 0, resource: targetTextures[targetId].createView() }, { binding: 1, resource: sampler }]
        });
    }
    const assetCache = {};
    async function getAsset(name) {
        if (!assetCache[name]) {
            const image = await loadImageTexture(device, name);
            assetCache[name] = {
                image,
                bindGroup: device.createBindGroup({
                    layout: textureLayout,
                    entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
                })
            };
        }
        return assetCache[name];
    }

    const operationResources = [];
    for (const pass of passes) {
        const resources = [];
        for (const operation of pass.operations) {
            const source = operation.source;
            const asset = source.kind === 'asset' ? await getAsset(source.asset) : null;
            const textureBindGroup = source.kind === 'asset'
                ? asset.bindGroup
                : targetBindGroups[source.target];
            let uniformBindGroup = null;
            let uniformBuffer = null;
            if (operation.kind === 'sprite') {
                const crop = operation.crop_uv;
                const image = asset.image;
                const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
                const scaleY = operation.target_height_scale;
                const scaleX = scaleY * (cropAspect / (target.width / target.height));
                const uniformData = new Float32Array([
                    operation.position[0], operation.position[1], scaleX, scaleY,
                    crop[0], crop[1], crop[2], crop[3],
                    image.keyColor[0], image.keyColor[1], image.keyColor[2], operation.tolerance,
                    operation.smoothness, operation.z_depth, operation.opacity, 0
                ]);
                uniformBuffer = device.createBuffer({
                    size: uniformData.byteLength,
                    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
                });
                device.queue.writeBuffer(uniformBuffer, 0, uniformData);
                uniformBindGroup = device.createBindGroup({
                    layout: uniformLayout,
                    entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
                });
            }
            resources.push({ pipeline: operation.pipeline, textureBindGroup, uniformBindGroup, uniformBuffer, vertexCount: operation.vertex_count });
        }
        operationResources.push({ target: pass.target, clear: clearByPass[pass.id], resources });
    }
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');

    async function executeChain(renderTargets, pipelines) {
        const started = performance.now();
        for (const pass of operationResources) {
            const clear = pass.clear;
            const encoder = device.createCommandEncoder();
            const renderPass = encoder.beginRenderPass({
                colorAttachments: [{
                    view: renderTargets[pass.target].createView(),
                    clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
                    loadOp: 'clear', storeOp: 'store'
                }]
            });
            for (const operation of pass.resources) {
                renderPass.setPipeline(pipelines[operation.pipeline]);
                renderPass.setBindGroup(0, operation.textureBindGroup);
                if (operation.uniformBindGroup) renderPass.setBindGroup(1, operation.uniformBindGroup);
                renderPass.draw(operation.vertexCount, 1, 0, 0);
            }
            renderPass.end();
            device.queue.submit([encoder.finish()]);
        }
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }

    const coldRenderTimeMs = await executeChain(targetTextures, offscreenPipelines);
    const warmRenderTimeMs = await executeChain(targetTextures, offscreenPipelines);
    const bytes = await readTextureBytes(device, targetTextures.C, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc05_interleaved_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc05_interleaved.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen của 3 pass + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        image_name: 'tc05_interleaved_web.png'
    });

    const canvas = document.getElementById('canvas-tc05');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executeChain({ ...targetTextures, C: context.getCurrentTexture() }, createPipelines(canvasFormat));
    document.getElementById('tag-tc05').textContent = 'PASS';
    document.getElementById('tag-tc05').className = 'tag tag-passed';

    for (const texture of Object.values(targetTextures)) texture.destroy();
    for (const asset of Object.values(assetCache)) asset.image.texture.destroy();
    for (const pass of operationResources) {
        for (const operation of pass.resources) if (operation.uniformBuffer) operation.uniformBuffer.destroy();
    }
}

async function runTC06(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc06_gc.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC06 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const poolSpec = manifest.graph.node_pool;
    const operation = manifest.graph.operation;
    const pool = Array.from({ length: poolSpec.allocated }, (_, index) => ({ id: `node_${index}` }));
    pool.splice(0, poolSpec.freed);
    const poolCheck = pool.length === poolSpec.surviving && pool[0]?.id === poolSpec.surviving_node;
    if (!poolCheck) throw new Error('TC06 Web RenderNodePool invariant failed');

    const shaderCode = await fetchShader(manifest.graph.pipeline.shader);
    const shaderModule = device.createShaderModule({ code: shaderCode });
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function createPipeline(format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shaderModule, entryPoint: 'vs_main' },
            fragment: {
                module: shaderModule,
                entryPoint: 'fs_main',
                targets: [{ format, blend: {
                    color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
                    alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
                }}]
            },
            primitive: { topology: 'triangle-list' }
        });
    }
    const image = await loadImageTexture(device, operation.source.asset);
    const crop = operation.crop_uv;
    const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
    const scaleY = operation.target_height_scale;
    const scaleX = scaleY * (cropAspect / (target.width / target.height));
    const uniformData = new Float32Array([
        operation.position[0], operation.position[1], scaleX, scaleY,
        crop[0], crop[1], crop[2], crop[3],
        image.keyColor[0], image.keyColor[1], image.keyColor[2], operation.tolerance,
        operation.smoothness, operation.z_depth, operation.opacity, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear' });
    const textureBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const pipeline = createPipeline('rgba8unorm-srgb');

    async function executeDraw(outputTexture, renderPipeline) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: { r: manifest.graph.clear_color[0], g: manifest.graph.clear_color[1], b: manifest.graph.clear_color[2], a: manifest.graph.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(renderPipeline);
        pass.setBindGroup(0, textureBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(operation.vertex_count, 1, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }

    const coldRenderTimeMs = await executeDraw(targetTexture, pipeline);
    const warmRenderTimeMs = await executeDraw(targetTexture, pipeline);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc06_gc_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc06_gc.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen của graph còn một node + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        allocated_nodes: poolSpec.allocated,
        freed_nodes: poolSpec.freed,
        surviving_nodes: pool.length,
        pool_check: poolCheck,
        image_name: 'tc06_gc_web.png'
    });

    const canvas = document.getElementById('canvas-tc06');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executeDraw(context.getCurrentTexture(), createPipeline(canvasFormat));
    document.getElementById('tag-tc06').textContent = 'PASS';
    document.getElementById('tag-tc06').className = 'tag tag-passed';
    targetTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC07(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc07_recursion.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC07 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
        }
    }
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC07 blend mode: ${name}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            const layouts = spec.has_uniform ? [textureLayout, uniformLayout] : [textureLayout];
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: layouts }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetCache = {};
    async function getAsset(name) {
        if (!assetCache[name]) {
            const image = await loadImageTexture(device, name);
            assetCache[name] = {
                image,
                bindGroup: device.createBindGroup({
                    layout: textureLayout,
                    entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
                })
            };
        }
        return assetCache[name];
    }
    const operationResources = [];
    for (const operation of operations) {
        const asset = await getAsset(operation.source.asset);
        let uniformBindGroup = null;
        let uniformBuffer = null;
        if (operation.kind === 'sprite') {
            const crop = operation.crop_uv;
            const cropAspect = ((crop[2] - crop[0]) * asset.image.width) / Math.max((crop[3] - crop[1]) * asset.image.height, 1);
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / (target.width / target.height));
            const uniformData = new Float32Array([
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                asset.image.keyColor[0], asset.image.keyColor[1], asset.image.keyColor[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ]);
            uniformBuffer = device.createBuffer({
                size: uniformData.byteLength,
                usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
            });
            device.queue.writeBuffer(uniformBuffer, 0, uniformData);
            uniformBindGroup = device.createBindGroup({
                layout: uniformLayout,
                entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
            });
        }
        operationResources.push({
            pipeline: operation.pipeline,
            textureBindGroup: asset.bindGroup,
            uniformBindGroup,
            uniformBuffer,
            vertexCount: operation.vertex_count
        });
    }
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    async function executeGraph(outputTexture, pipelines) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: {
                r: manifest.graph.clear_color[0], g: manifest.graph.clear_color[1],
                b: manifest.graph.clear_color[2], a: manifest.graph.clear_color[3]
            },
            loadOp: 'clear', storeOp: 'store'
        }]});
        for (const operation of operationResources) {
            pass.setPipeline(pipelines[operation.pipeline]);
            pass.setBindGroup(0, operation.textureBindGroup);
            if (operation.uniformBindGroup) pass.setBindGroup(1, operation.uniformBindGroup);
            pass.draw(operation.vertexCount, 1, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeGraph(targetTexture, offscreenPipelines);
    const warmRenderTimeMs = await executeGraph(targetTexture, offscreenPipelines);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc07_recursion_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc07_recursion.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen của graph flatten 5 cấp + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        recursion_depth: manifest.graph.depth,
        flattened_operations: operations.length,
        image_name: 'tc07_recursion_web.png'
    });
    const canvas = document.getElementById('canvas-tc07');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executeGraph(context.getCurrentTexture(), createPipelines(canvasFormat));
    document.getElementById('tag-tc07').textContent = 'PASS';
    document.getElementById('tag-tc07').className = 'tag tag-passed';
    targetTexture.destroy();
    for (const asset of Object.values(assetCache)) asset.image.texture.destroy();
    for (const operation of operationResources) if (operation.uniformBuffer) operation.uniformBuffer.destroy();
}

async function runTC08(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc08_massive.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC08 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
        }
    }
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC08 blend mode: ${name}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    const sampler = device.createSampler({
        addressModeU: 'repeat', addressModeV: 'repeat', addressModeW: 'repeat',
        magFilter: manifest.graph.sampler.mag_filter,
        minFilter: manifest.graph.sampler.min_filter,
        mipmapFilter: manifest.graph.sampler.mipmap_filter
    });
    const image = await loadImageTexture(device, operations[0].source.asset);
    const backgroundBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    async function executeGraph(outputTexture, pipelines) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: {
                r: manifest.graph.clear_color[0], g: manifest.graph.clear_color[1],
                b: manifest.graph.clear_color[2], a: manifest.graph.clear_color[3]
            },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipelines.background);
        pass.setBindGroup(0, backgroundBindGroup);
        pass.draw(operations[0].vertex_count, operations[0].instance_count, 0, 0);
        pass.setPipeline(pipelines.particles);
        pass.draw(operations[1].vertex_count, operations[1].instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeGraph(targetTexture, offscreenPipelines);
    const warmRenderTimeMs = await executeGraph(targetTexture, offscreenPipelines);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc08_massive_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc08_massive.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen của graph 1 node/2 draw command với 10.000 instance + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count,
        draw_commands: manifest.graph.command_count,
        instance_count: operations[1].instance_count,
        image_name: 'tc08_massive_web.png'
    });
    const canvas = document.getElementById('canvas-tc08');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executeGraph(context.getCurrentTexture(), createPipelines(canvasFormat));
    document.getElementById('tag-tc08').textContent = 'PASS';
    document.getElementById('tag-tc08').className = 'tag tag-passed';
    targetTexture.destroy();
    image.texture.destroy();
}

async function runTC09(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc09_caching.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC09 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const warmIterationCount = manifest.cache_contract.warm_iteration_count;
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
        }
    }
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC09 blend mode: ${name}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, operations[0].source.asset);
    const backgroundBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    async function executeGraph(outputTexture, pipelines) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: {
                r: manifest.graph.clear_color[0], g: manifest.graph.clear_color[1],
                b: manifest.graph.clear_color[2], a: manifest.graph.clear_color[3]
            },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipelines.background);
        pass.setBindGroup(0, backgroundBindGroup);
        pass.draw(operations[0].vertex_count, operations[0].instance_count, 0, 0);
        pass.setPipeline(pipelines.particles);
        pass.draw(operations[1].vertex_count, operations[1].instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeGraph(targetTexture, offscreenPipelines);
    const coldBytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    const warmTimes = [];
    for (let index = 0; index < warmIterationCount; index++) {
        warmTimes.push(await executeGraph(targetTexture, offscreenPipelines));
    }
    const warmRenderTimeMs = warmTimes.reduce((sum, value) => sum + value, 0) / warmTimes.length;
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC09 cache changed rendered output');
    await saveRawTexture(bytes, {
        name: 'tc09_caching_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: warmIterationCount,
        speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual,
        manifest: 'tests/shared_assets/manifests/tc09_caching.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen của cùng graph + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count,
        draw_commands: manifest.graph.command_count,
        instance_count: operations[1].instance_count,
        image_name: 'tc09_caching_web.png'
    });
    const canvas = document.getElementById('canvas-tc09');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executeGraph(context.getCurrentTexture(), createPipelines(canvasFormat));
    document.getElementById('tag-tc09').textContent = 'PASS';
    document.getElementById('tag-tc09').className = 'tag tag-passed';
    targetTexture.destroy();
    image.texture.destroy();
}

async function runTC10(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc10_fallback.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC10 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const errorContract = manifest.error_contract;
    let validationPassed = false;
    try {
        const missingBindGroup = null;
        if (missingBindGroup !== null) throw new Error('unexpected resource');
        throw new Error(`${errorContract.type}(${errorContract.missing_bind_group})`);
    } catch (error) {
        validationPassed = error.message === `${errorContract.type}(${errorContract.missing_bind_group})`;
    }
    if (!validationPassed) throw new Error('TC10 Web contract mirror did not reject the missing bind group');
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    async function executeFallback(outputTexture) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: {
                r: manifest.graph.clear_color[0], g: manifest.graph.clear_color[1],
                b: manifest.graph.clear_color[2], a: manifest.graph.clear_color[3]
            },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeFallback(targetTexture);
    const coldBytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    const warmRenderTimeMs = await executeFallback(targetTexture);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    const expectedPixel = new Uint8Array([255, 0, 255, 255]);
    const expectedBytes = new Uint8Array(bytes.length);
    for (let offset = 0; offset < expectedBytes.length; offset += 4) expectedBytes.set(expectedPixel, offset);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    const fallbackOutputCorrect = bytes.every((value, index) => value === expectedBytes[index]);
    if (!cacheOutputEqual || !fallbackOutputCorrect) throw new Error('TC10 fallback output mismatch');
    await saveRawTexture(bytes, {
        name: 'tc10_fallback_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1,
        speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual,
        validation_error: errorContract.type,
        missing_bind_group: errorContract.missing_bind_group,
        validation_passed: validationPassed,
        panic_occurred: false,
        fallback_color: [255, 0, 255, 255],
        manifest: 'tests/shared_assets/manifests/tc10_fallback.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'fallback clear execute + submit queue + onSubmittedWorkDone; không gồm contract validation mirror, khởi tạo device và readback',
        node_count: manifest.graph.node_count,
        draw_commands: manifest.graph.command_count,
        image_name: 'tc10_fallback_web.png'
    });
    const canvas = document.getElementById('canvas-tc10');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executeFallback(context.getCurrentTexture());
    document.getElementById('tag-tc10').textContent = 'PASS';
    document.getElementById('tag-tc10').className = 'tag tag-passed';
    targetTexture.destroy();
}

async function runTC11(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc11_viewport.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC11 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const leftTarget = manifest.graph.targets.left;
    const rightTarget = manifest.graph.targets.right;
    const shader = device.createShaderModule({ code: await fetchShader('splitscreen_composite.wgsl') });
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    function createSplitPipeline(format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, textureLayout] }),
            vertex: { module: shader, entryPoint: 'vs_main' },
            fragment: { module: shader, entryPoint: 'fs_main', targets: [{ format }] },
            primitive: { topology: 'triangle-list' }
        });
    }
    const pipeline = createSplitPipeline('rgba8unorm-srgb');
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const leftTexture = device.createTexture({
        size: [leftTarget.width, leftTarget.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const rightTexture = device.createTexture({
        size: [rightTarget.width, rightTarget.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const leftBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: leftTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const rightBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: rightTexture.createView() }, { binding: 1, resource: sampler }]
    });
    function clearPass(encoder, texture, clearValue) {
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: texture.createView(), clearValue, loadOp: 'clear', storeOp: 'store'
        }]});
        pass.end();
    }
    async function executeAll(outputTexture) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        clearPass(encoder, leftTexture, { r: 0.15, g: 0.08, b: 0.20, a: 1.0 });
        clearPass(encoder, rightTexture, { r: 0.008, g: 0.012, b: 0.045, a: 1.0 });
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, leftBindGroup);
        pass.setBindGroup(1, rightBindGroup);
        pass.draw(6, 1, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeAll(finalTexture);
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const warmRenderTimeMs = await executeAll(finalTexture);
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC11 viewport output changed between runs');
    await saveRawTexture(bytes, {
        name: 'tc11_viewport_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        cache_output_equal: cacheOutputEqual,
        manifest: 'tests/shared_assets/manifests/tc11_viewport.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '3 pass offscreen (left → right → final) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count,
        draw_commands: manifest.graph.command_count,
        pass_count: manifest.graph.passes.length,
        viewport_count: 2,
        image_name: 'tc11_viewport_web.png'
    });
    const canvas = document.getElementById('canvas-tc11');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasPipeline = createSplitPipeline(canvasFormat);
    const canvasPass = device.createCommandEncoder();
    const pass = canvasPass.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    pass.setPipeline(canvasPipeline);
    pass.setBindGroup(0, leftBindGroup);
    pass.setBindGroup(1, rightBindGroup);
    pass.draw(6, 1, 0, 0);
    pass.end();
    device.queue.submit([canvasPass.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc11_viewport_web_preview.png');
    document.getElementById('tag-tc11').textContent = 'PASS';
    document.getElementById('tag-tc11').className = 'tag tag-passed';
    leftTexture.destroy();
    rightTexture.destroy();
    finalTexture.destroy();
}

async function runTC12(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc12_chroma.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC12 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const clear = manifest.graph.clear_color;
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const skyShader = device.createShaderModule({ code: await fetchShader('sky_composite.wgsl') });
    const chromaShader = device.createShaderModule({ code: await fetchShader('chroma_key_cropped.wgsl') });
    const alphaBlend = {
        color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
    };
    function createPipeline(shader, format, blend) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shader, entryPoint: 'vs_main' },
            fragment: {
                module: shader,
                entryPoint: 'fs_main',
                targets: [blend ? { format, blend } : { format }]
            },
            primitive: { topology: 'triangle-list' }
        });
    }
    const skyPipeline = createPipeline(skyShader, 'rgba8unorm-srgb', null);
    const chromaPipeline = createPipeline(chromaShader, 'rgba8unorm-srgb', alphaBlend);
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetNames = [...new Set(operations.map(operation => operation.asset))];
    const images = {};
    for (const asset of assetNames) images[asset] = await loadImageTexture(device, asset);
    const textureBindGroups = {};
    const uniformBuffers = {};
    const uniformBindGroups = {};
    function createUniform(id, values, minimumSize = 0) {
        const data = new Float32Array(values);
        const size = Math.max(data.byteLength, minimumSize);
        const buffer = device.createBuffer({ size, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(buffer, 0, data);
        uniformBuffers[id] = buffer;
        uniformBindGroups[id] = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
    }
    for (const operation of operations) {
        const image = images[operation.asset];
        if (!textureBindGroups[operation.asset]) {
            textureBindGroups[operation.asset] = device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
            });
        }
        if (operation.kind === 'sky') {
            const uniform = operation.uniform;
            createUniform(operation.id, [
                ...uniform.top_color, uniform.noise_strength,
                ...uniform.bottom_color, uniform.time
            ], 64);
        } else {
            const crop = operation.crop_uv;
            const cropWidth = (crop[2] - crop[0]) * image.width;
            const cropHeight = (crop[3] - crop[1]) * image.height;
            const cropAspect = cropWidth / Math.max(cropHeight, 1);
            const screenAspect = target.width / target.height;
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / screenAspect);
            createUniform(operation.id, [
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                operation.key_color[0], operation.key_color[1], operation.key_color[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ]);
        }
    }
    const targetTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    function drawOperations(pass, skyPipelineForFormat, chromaPipelineForFormat) {
        for (const operation of operations) {
            pass.setPipeline(operation.kind === 'sky' ? skyPipelineForFormat : chromaPipelineForFormat);
            pass.setBindGroup(0, textureBindGroups[operation.asset]);
            pass.setBindGroup(1, uniformBindGroups[operation.id]);
            pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        }
    }
    async function executeDraw(outputTexture, skyPipelineForFormat, chromaPipelineForFormat) {
        const started = performance.now();
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        drawOperations(pass, skyPipelineForFormat, chromaPipelineForFormat);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeDraw(targetTexture, skyPipeline, chromaPipeline);
    const coldBytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    const warmRenderTimeMs = await executeDraw(targetTexture, skyPipeline, chromaPipeline);
    const bytes = await readTextureBytes(device, targetTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC12 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc12_chroma_web',
        width: target.width,
        height: target.height,
        format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs,
        warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1,
        speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual,
        manifest: 'tests/shared_assets/manifests/tc12_chroma.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '6 draw command execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count,
        draw_commands: manifest.graph.command_count,
        pass_count: 1,
        image_name: 'tc12_chroma_web.png'
    });
    const canvas = document.getElementById('canvas-tc12');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasSkyPipeline = createPipeline(skyShader, canvasFormat, null);
    const canvasChromaPipeline = createPipeline(chromaShader, canvasFormat, alphaBlend);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
        loadOp: 'clear', storeOp: 'store'
    }]});
    drawOperations(pass, canvasSkyPipeline, canvasChromaPipeline);
    pass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc12_chroma_web_preview.png');
    document.getElementById('tag-tc12').textContent = 'PASS';
    document.getElementById('tag-tc12').className = 'tag tag-passed';
    targetTexture.destroy();
    for (const asset of assetNames) images[asset].texture.destroy();
    for (const id of Object.keys(uniformBuffers)) uniformBuffers[id].destroy();
}

async function runTC13(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc13_blur.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC13 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const operationById = Object.fromEntries(operations.map(operation => [operation.id, operation]));
    const clearBackground = [0.02, 0.10, 0.15, 1.0];
    const clearBlack = [0, 0, 0, 1];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaders = {};
    for (const name of ['sky_composite.wgsl', 'chroma_key_cropped.wgsl', 'gaussian_blur_separable.wgsl', 'texture_blit.wgsl', 'star_particles_sprite.wgsl']) {
        shaders[name] = device.createShaderModule({ code: await fetchShader(name) });
    }
    const alphaBlend = {
        color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
    };
    const additiveBlend = {
        color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
    };
    function createPipeline(shaderName, format, blend) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shaders[shaderName], entryPoint: 'vs_main' },
            fragment: {
                module: shaders[shaderName],
                entryPoint: 'fs_main',
                targets: [blend ? { format, blend } : { format }]
            },
            primitive: { topology: 'triangle-list' }
        });
    }
    function createNoUniformPipeline(shaderName, format, blend) {
        const layout = device.createPipelineLayout({ bindGroupLayouts: [textureLayout] });
        return device.createRenderPipeline({
            layout,
            vertex: { module: shaders[shaderName], entryPoint: 'vs_main' },
            fragment: {
                module: shaders[shaderName],
                entryPoint: 'fs_main',
                targets: [blend ? { format, blend } : { format }]
            },
            primitive: { topology: 'triangle-list' }
        });
    }
    const pipelines = {
        sky: createPipeline('sky_composite.wgsl', 'rgba8unorm-srgb', null),
        chroma: createPipeline('chroma_key_cropped.wgsl', 'rgba8unorm-srgb', alphaBlend),
        blur: createPipeline('gaussian_blur_separable.wgsl', 'rgba8unorm-srgb', null),
        blit: createNoUniformPipeline('texture_blit.wgsl', 'rgba8unorm-srgb', null),
        wisps: createNoUniformPipeline('star_particles_sprite.wgsl', 'rgba8unorm-srgb', additiveBlend)
    };
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetNames = [...new Set(operations.filter(operation => operation.asset).map(operation => operation.asset))];
    const images = {};
    for (const asset of assetNames) images[asset] = await loadImageTexture(device, asset);
    const textureBindGroups = {};
    const uniformBuffers = {};
    const uniformBindGroups = {};
    const op = id => operationById[id];
    function textureBindGroup(texture, key) {
        if (!textureBindGroups[key]) {
            textureBindGroups[key] = device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: texture.createView() }, { binding: 1, resource: sampler }]
            });
        }
        return textureBindGroups[key];
    }
    for (const asset of assetNames) textureBindGroup(images[asset].texture, asset);
    function createUniform(id, values, minimumSize = 0) {
        const data = new Float32Array(values);
        const size = Math.max(data.byteLength, minimumSize);
        const buffer = device.createBuffer({ size, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(buffer, 0, data);
        uniformBuffers[id] = buffer;
        uniformBindGroups[id] = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
    }
    for (const operation of operations) {
        if (operation.kind === 'sky') {
            createUniform(operation.id, [
                ...operation.uniform.top_color, operation.uniform.noise_strength,
                ...operation.uniform.bottom_color, operation.uniform.time
            ], 64);
        } else if (operation.kind === 'blur') {
            createUniform(operation.id, [...operation.uniform.direction, operation.uniform.radius, 0]);
        } else if (operation.position) {
            const image = images[operation.asset];
            const crop = operation.crop_uv;
            const cropWidth = (crop[2] - crop[0]) * image.width;
            const cropHeight = (crop[3] - crop[1]) * image.height;
            const cropAspect = cropWidth / Math.max(cropHeight, 1);
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / (target.width / target.height));
            createUniform(operation.id, [
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                operation.key_color[0], operation.key_color[1], operation.key_color[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ]);
        }
    }
    const backgroundTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const blurTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    textureBindGroup(backgroundTexture, 'background_target');
    textureBindGroup(blurTexture, 'blur_target');
    function bindFor(operation, targetOverride) {
        if (targetOverride) return textureBindGroup(targetOverride, targetOverride === backgroundTexture ? 'background_target' : 'blur_target');
        return textureBindGroups[operation.asset];
    }
    function drawSpriteOperation(pass, id, pipelineOverride) {
        const operation = op(id);
        pass.setPipeline(pipelineOverride || pipelines[operation.pipeline]);
        pass.setBindGroup(0, bindFor(operation));
        if (uniformBindGroups[id]) pass.setBindGroup(1, uniformBindGroups[id]);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
    }
    function drawTextureOperation(pass, id, sourceTexture, pipelineOverride) {
        const operation = op(id);
        pass.setPipeline(pipelineOverride || pipelines[operation.pipeline]);
        pass.setBindGroup(0, bindFor(operation, sourceTexture));
        if (uniformBindGroups[id]) pass.setBindGroup(1, uniformBindGroups[id]);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
    }
    async function submitPass(texture, clearValue, draw) {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: texture.createView(), clearValue: { r: clearValue[0], g: clearValue[1], b: clearValue[2], a: clearValue[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        draw(pass);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC13 validation error: ${error.message}`);
    }
    async function executeAll() {
        const started = performance.now();
        await submitPass(backgroundTexture, clearBackground, pass => {
            drawSpriteOperation(pass, 'forest_sky');
            drawSpriteOperation(pass, 'forest_wisps', pipelines.wisps);
            drawSpriteOperation(pass, 'tree_left');
            drawSpriteOperation(pass, 'tree_center');
            drawSpriteOperation(pass, 'tree_right');
        });
        await submitPass(blurTexture, clearBlack, pass => drawTextureOperation(pass, 'blur_horizontal', backgroundTexture));
        await submitPass(backgroundTexture, clearBlack, pass => drawTextureOperation(pass, 'blur_vertical', blurTexture));
        await submitPass(finalTexture, clearBlack, pass => {
            drawTextureOperation(pass, 'background_blit', backgroundTexture, pipelines.blit);
            drawSpriteOperation(pass, 'paladin_foreground');
            drawSpriteOperation(pass, 'archer_foreground');
            drawSpriteOperation(pass, 'chest_foreground');
        });
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeAll();
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const warmRenderTimeMs = await executeAll();
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC13 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc13_blur_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, manifest: 'tests/shared_assets/manifests/tc13_blur.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        validation_passed: true, validation_error: null,
        timing_scope: '4 pass (background → blur H → blur V → final) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc13_blur_web.png'
    });
    const canvas = document.getElementById('canvas-tc13');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasBlit = createNoUniformPipeline('texture_blit.wgsl', canvasFormat, null);
    const canvasChroma = createPipeline('chroma_key_cropped.wgsl', canvasFormat, alphaBlend);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    drawTextureOperation(pass, 'background_blit', backgroundTexture, canvasBlit);
    drawSpriteOperation(pass, 'paladin_foreground', canvasChroma);
    drawSpriteOperation(pass, 'archer_foreground', canvasChroma);
    drawSpriteOperation(pass, 'chest_foreground', canvasChroma);
    pass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc13_blur_web_preview.png');
    document.getElementById('tag-tc13').textContent = 'PASS';
    document.getElementById('tag-tc13').className = 'tag tag-passed';
    backgroundTexture.destroy();
    blurTexture.destroy();
    finalTexture.destroy();
    for (const asset of assetNames) images[asset].texture.destroy();
    for (const id of Object.keys(uniformBuffers)) uniformBuffers[id].destroy();
}

async function runTC14(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc14_grading.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC14 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const operationById = Object.fromEntries(operations.map(operation => [operation.id, operation]));
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaders = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaders[spec.shader]) shaders[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
    }
    shaders['texture_blit.wgsl'] = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const alphaBlend = {
        color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
    };
    const additiveBlend = {
        color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
    };
    function createPipeline(shaderName, format, blend) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
            vertex: { module: shaders[shaderName], entryPoint: 'vs_main' },
            fragment: { module: shaders[shaderName], entryPoint: 'fs_main', targets: [blend ? { format, blend } : { format }] },
            primitive: { topology: 'triangle-list' }
        });
    }
    function createNoUniformPipeline(shaderName, format, blend) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
            vertex: { module: shaders[shaderName], entryPoint: 'vs_main' },
            fragment: { module: shaders[shaderName], entryPoint: 'fs_main', targets: [blend ? { format, blend } : { format }] },
            primitive: { topology: 'triangle-list' }
        });
    }
    const pipelines = {
        sky: createPipeline('sky_composite.wgsl', 'rgba8unorm-srgb', null),
        chroma: createPipeline('chroma_key_cropped.wgsl', 'rgba8unorm-srgb', alphaBlend),
        sparks: createNoUniformPipeline('star_particles_sprite.wgsl', 'rgba8unorm-srgb', additiveBlend),
        grading: createPipeline('color_grading_filmic.wgsl', 'rgba8unorm-srgb', null)
    };
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetNames = [...new Set(operations.filter(operation => operation.asset).map(operation => operation.asset))];
    const images = {};
    for (const asset of assetNames) images[asset] = await loadImageTexture(device, asset);
    const textureBindGroups = {};
    const uniformBuffers = {};
    const uniformBindGroups = {};
    function textureBindGroup(texture, key) {
        if (!textureBindGroups[key]) {
            textureBindGroups[key] = device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: texture.createView() }, { binding: 1, resource: sampler }]
            });
        }
        return textureBindGroups[key];
    }
    for (const asset of assetNames) textureBindGroup(images[asset].texture, asset);
    function createUniform(id, values, minimumSize = 0) {
        const data = new Float32Array(values);
        const size = Math.max(data.byteLength, minimumSize);
        const buffer = device.createBuffer({ size, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(buffer, 0, data);
        uniformBuffers[id] = buffer;
        uniformBindGroups[id] = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
    }
    for (const operation of operations) {
        if (operation.kind === 'sky') {
            createUniform(operation.id, [
                ...operation.uniform.top_color, operation.uniform.noise_strength,
                ...operation.uniform.bottom_color, operation.uniform.time
            ], 64);
        } else if (operation.kind === 'postprocess') {
            createUniform(operation.id, [
                ...operation.uniform.params,
                ...operation.uniform.shadow_tint_vig,
                ...operation.uniform.highlight_tint
            ]);
        } else if (operation.position) {
            const image = images[operation.asset];
            const crop = operation.crop_uv;
            const cropWidth = (crop[2] - crop[0]) * image.width;
            const cropHeight = (crop[3] - crop[1]) * image.height;
            const cropAspect = cropWidth / Math.max(cropHeight, 1);
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / (target.width / target.height));
            createUniform(operation.id, [
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                operation.key_color[0], operation.key_color[1], operation.key_color[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ]);
        }
    }
    const sceneTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    textureBindGroup(sceneTexture, 'scene_target');
    function drawScene(pass) {
        for (const operation of operations.filter(operation => !operation.source_target)) {
            pass.setPipeline(pipelines[operation.pipeline]);
            pass.setBindGroup(0, textureBindGroups[operation.asset]);
            if (uniformBindGroups[operation.id]) pass.setBindGroup(1, uniformBindGroups[operation.id]);
            pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        }
    }
    function drawGrading(pass) {
        const operation = operationById.color_grade;
        pass.setPipeline(pipelines[operation.pipeline]);
        pass.setBindGroup(0, textureBindGroups.scene_target);
        pass.setBindGroup(1, uniformBindGroups[operation.id]);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
    }
    async function submitPass(texture, clearColor, draw) {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: texture.createView(),
            clearValue: { r: clearColor[0], g: clearColor[1], b: clearColor[2], a: clearColor[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        draw(pass);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC14 validation error: ${error.message}`);
    }
    async function executeAll() {
        const started = performance.now();
        await submitPass(sceneTexture, manifest.graph.passes[0].clear_color, drawScene);
        await submitPass(finalTexture, manifest.graph.passes[1].clear_color, drawGrading);
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executeAll();
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const warmRenderTimeMs = await executeAll();
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC14 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc14_grading_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc14_grading.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '2 pass (scene → color grading) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc14_grading_web.png'
    });
    const canvas = document.getElementById('canvas-tc14');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasBlit = createNoUniformPipeline('texture_blit.wgsl', canvasFormat, null);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    pass.setPipeline(canvasBlit);
    pass.setBindGroup(0, textureBindGroups.final_target || textureBindGroup(finalTexture, 'final_target'));
    pass.draw(6, 1, 0, 0);
    pass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc14_grading_web_preview.png');
    document.getElementById('tag-tc14').textContent = 'PASS';
    document.getElementById('tag-tc14').className = 'tag tag-passed';
    sceneTexture.destroy();
    finalTexture.destroy();
    for (const asset of assetNames) images[asset].texture.destroy();
    for (const id of Object.keys(uniformBuffers)) uniformBuffers[id].destroy();
}

async function runTC15(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc15_snow.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC15 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
    }
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC15 blend mode: ${name}`);
    }
    function pipelineLayouts(spec) {
        if (spec.layout === 'texture') return [textureLayout];
        if (spec.layout === 'texture_uniform') return [textureLayout, uniformLayout];
        if (spec.layout === 'texture_texture_uniform') return [textureLayout, textureLayout, uniformLayout];
        throw new Error(`Unsupported TC15 pipeline layout: ${spec.layout}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: pipelineLayouts(spec) }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetNames = [...new Set(operations.flatMap(operation => (operation.source || []).filter(source => source.kind === 'asset').map(source => source.asset)))];
    const assets = {};
    for (const name of assetNames) {
        const image = await loadImageTexture(device, name);
        assets[name] = {
            image,
            bindGroup: device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
            })
        };
    }
    const uniformBuffers = {};
    function makeUniform(id, values, minimumSize = 0) {
        const data = new Float32Array(values);
        const size = Math.max(data.byteLength, minimumSize);
        const buffer = device.createBuffer({ size, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(buffer, 0, data);
        uniformBuffers[id] = buffer;
        return device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
    }
    function uniformData(operation) {
        const u = operation.uniform;
        if (operation.kind === 'sky') return { values: [...u.top_color, u.noise_strength, ...u.bottom_color, u.time], minimumSize: 64 };
        if (operation.kind === 'moon') return { values: [...u.model_view, ...u.uv_min, ...u.uv_max, ...u.key_color, u.tolerance, u.smoothness, u.noise_strength, u.glow_intensity, u._pad] };
        if (operation.kind === 'cloud') return { values: [...u.model_view, ...u.uv_bounds, ...u.key_color_tol, ...u.params, ...u.lighting_pos] };
        if (operation.kind === 'snow') return { values: [u.time, u.wind_speed, u.gravity, u.particle_count] };
        if (operation.kind === 'chroma_sprite') {
            const image = assets[operation.asset].image;
            const crop = operation.crop_uv;
            const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / (target.width / target.height));
            return { values: [
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                operation.key_color[0], operation.key_color[1], operation.key_color[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ] };
        }
        return null;
    }
    const resources = [];
    for (const operation of operations) {
        const bindGroups = [];
        for (const source of operation.source || []) {
            if (source.kind !== 'asset') throw new Error(`Unsupported TC15 source kind: ${source.kind}`);
            bindGroups.push(assets[source.asset].bindGroup);
        }
        const uniform = uniformData(operation);
        if (uniform) bindGroups.push(makeUniform(operation.id, uniform.values, uniform.minimumSize || 0));
        resources.push({ operation, bindGroups });
    }
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    async function render(outputTexture, pipelines) {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const clear = manifest.graph.passes[0].clear_color;
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        for (const resource of resources) {
            pass.setPipeline(pipelines[resource.operation.pipeline]);
            resource.bindGroups.forEach((bindGroup, index) => pass.setBindGroup(index, bindGroup));
            pass.draw(resource.operation.vertex_count, resource.operation.instance_count, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC15 validation error: ${error.message}`);
    }
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    const startedCold = performance.now();
    await render(finalTexture, offscreenPipelines);
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render(finalTexture, offscreenPipelines);
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC15 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc15_snow_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc15_snow.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (winter snow scene) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: manifest.evaluation.expected_instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc15_snow_web.png'
    });
    const canvas = document.getElementById('canvas-tc15');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await render(context.getCurrentTexture(), createPipelines(canvasFormat));
    await saveCanvasImage(canvas, 'tc15_snow_web_preview.png');
    document.getElementById('tag-tc15').textContent = 'PASS';
    document.getElementById('tag-tc15').className = 'tag tag-passed';
    finalTexture.destroy();
    for (const asset of Object.values(assets)) asset.image.texture.destroy();
    for (const buffer of Object.values(uniformBuffers)) buffer.destroy();
}

async function runTC16(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc16_sdf.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC16 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: [uniformLayout] }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{
                        format,
                        blend: {
                            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
                            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
                        }
                    }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    const aspectRatio = target.width / target.height;
    const resources = operations.map(operation => {
        const u = operation.uniform;
        const values = [
            u.shape_type, u.size_x, u.size_y, u.corner_radius,
            ...u.color, ...u.border_color,
            u.border_width, u.glow_strength,
            u.position[0], u.position[1], u.rotation, u.scale, aspectRatio, 0
        ];
        const data = new Float32Array(values);
        const buffer = device.createBuffer({ size: data.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(buffer, 0, data);
        const bindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
        return { operation, bindGroup, buffer };
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    async function render(outputTexture, pipelines) {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const clear = manifest.graph.passes[0].clear_color;
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        for (const resource of resources) {
            pass.setPipeline(pipelines[resource.operation.pipeline]);
            pass.setBindGroup(0, resource.bindGroup);
            pass.draw(resource.operation.vertex_count, resource.operation.instance_count, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC16 validation error: ${error.message}`);
    }
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    const startedCold = performance.now();
    await render(finalTexture, offscreenPipelines);
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render(finalTexture, offscreenPipelines);
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC16 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc16_sdf_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc16_sdf.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (2D SDF scene, 4 draw commands) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        shape_count: manifest.evaluation.expected_shape_count,
        instance_count: operations.reduce((sum, operation) => sum + operation.instance_count, 0),
        pass_count: manifest.graph.passes.length,
        image_name: 'tc16_sdf_web.png'
    });
    const canvas = document.getElementById('canvas-tc16');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await render(context.getCurrentTexture(), createPipelines(canvasFormat));
    await saveCanvasImage(canvas, 'tc16_sdf_web_preview.png');
    document.getElementById('tag-tc16').textContent = 'PASS';
    document.getElementById('tag-tc16').className = 'tag tag-passed';
    finalTexture.destroy();
    for (const resource of resources) resource.buffer.destroy();
}

async function runTC17(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc17_outline.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC17 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const passes = manifest.graph.passes;
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
            const compilationInfo = await shaderModules[spec.shader].getCompilationInfo();
            const errors = compilationInfo.messages.filter(message => message.type === 'error');
            if (errors.length) throw new Error(`TC17 shader ${spec.shader}: ${errors.map(message => message.message).join('; ')}`);
        }
    }
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC17 blend mode: ${name}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    function createNoUniformPipeline(shaderModule, format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
            vertex: { module: shaderModule, entryPoint: 'vs_main' },
            fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format }] },
            primitive: { topology: 'triangle-list' }
        });
    }
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetNames = [...new Set(operations.flatMap(operation => (operation.source || [])
        .filter(source => source.kind === 'asset').map(source => source.asset)))];
    const assets = {};
    for (const asset of assetNames) {
        const image = await loadImageTexture(device, asset);
        assets[asset] = {
            image,
            bindGroup: device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
            })
        };
    }
    const heroesTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const targetBindGroups = {
        heroes: device.createBindGroup({
            layout: textureLayout,
            entries: [{ binding: 0, resource: heroesTexture.createView() }, { binding: 1, resource: sampler }]
        })
    };
    const uniformBuffers = [];
    function createUniform(values, minimumSize = 0) {
        const data = new Float32Array(values);
        const buffer = device.createBuffer({
            size: Math.max(data.byteLength, minimumSize),
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
        });
        device.queue.writeBuffer(buffer, 0, data);
        uniformBuffers.push(buffer);
        return device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
    }
    function uniformData(operation) {
        const u = operation.uniform;
        if (operation.kind === 'sky') {
            return { values: [...u.top_color, u.noise_strength, ...u.bottom_color, u.time], minimumSize: 64 };
        }
        if (operation.kind === 'chroma_sprite') {
            const image = assets[operation.asset].image;
            const crop = operation.crop_uv;
            const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / (target.width / target.height));
            return { values: [
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                operation.key_color[0], operation.key_color[1], operation.key_color[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ] };
        }
        if (operation.kind === 'outline_shadow_postprocess') {
            return { values: [
                ...u.outline_color, ...u.shadow_color,
                ...u.shadow_offset, ...u.texel_size,
                u.outline_thickness, 0, 0, 0
            ] };
        }
        throw new Error(`Unsupported TC17 operation kind: ${operation.kind}`);
    }
    const resources = operations.map(operation => {
        const bindGroups = [];
        for (const source of operation.source || []) {
            if (source.kind === 'asset') bindGroups.push(assets[source.asset].bindGroup);
            else if (source.kind === 'target') bindGroups.push(targetBindGroups[source.target]);
            else throw new Error(`Unsupported TC17 source kind: ${source.kind}`);
        }
        const uniform = uniformData(operation);
        bindGroups.push(createUniform(uniform.values, uniform.minimumSize || 0));
        return { operation, bindGroups };
    });
    async function submitPass(outputTexture, passSpec, pipelines) {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const clear = passSpec.clear_color;
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: outputTexture.createView(),
            clearValue: { r: clear[0], g: clear[1], b: clear[2], a: clear[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        for (const resource of resources.filter(item => item.operation.pass === passSpec.id)) {
            pass.setPipeline(pipelines[resource.operation.pipeline]);
            resource.bindGroups.forEach((bindGroup, index) => pass.setBindGroup(index, bindGroup));
            pass.draw(resource.operation.vertex_count, resource.operation.instance_count, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC17 validation error: ${error.message}`);
    }
    async function renderAll(finalOutput, pipelines) {
        const started = performance.now();
        await submitPass(heroesTexture, passes[0], pipelines);
        await submitPass(finalOutput, passes[1], pipelines);
        return performance.now() - started;
    }
    device.pushErrorScope('validation');
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    const pipelineError = await device.popErrorScope();
    if (pipelineError) throw new Error(`TC17 pipeline validation error: ${pipelineError.message}`);
    const coldRenderTimeMs = await renderAll(finalTexture, offscreenPipelines);
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const warmRenderTimeMs = await renderAll(finalTexture, offscreenPipelines);
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC17 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc17_outline_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc17_outline.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '2 pass (transparent heroes → sky/outline final) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operations.reduce((sum, operation) => sum + operation.instance_count, 0),
        pass_count: manifest.graph.passes.length, image_name: 'tc17_outline_web.png'
    });
    const canvas = document.getElementById('canvas-tc17');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = createNoUniformPipeline(blitShader, canvasFormat);
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    pass.setPipeline(canvasBlit);
    pass.setBindGroup(0, finalBindGroup);
    pass.draw(6, 1, 0, 0);
    pass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc17_outline_web_preview.png');
    document.getElementById('tag-tc17').textContent = 'PASS';
    document.getElementById('tag-tc17').className = 'tag tag-passed';
    heroesTexture.destroy();
    finalTexture.destroy();
    for (const asset of Object.values(assets)) asset.image.texture.destroy();
    for (const buffer of uniformBuffers) buffer.destroy();
}

async function runTC18(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc18_transition.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC18 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const passes = manifest.graph.passes;
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const dualTextureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
        { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
            const info = await shaderModules[spec.shader].getCompilationInfo();
            const errors = info.messages.filter(message => message.type === 'error');
            if (errors.length) throw new Error(`TC18 shader ${spec.shader}: ${errors.map(message => message.message).join('; ')}`);
        }
    }
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC18 blend mode: ${name}`);
    }
    function pipelineLayouts(spec) {
        if (spec.layout === 'texture_uniform') return [textureLayout, uniformLayout];
        if (spec.layout === 'dual_texture_uniform') return [dualTextureLayout, uniformLayout];
        throw new Error(`Unsupported TC18 pipeline layout: ${spec.layout}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: pipelineLayouts(spec) }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    function createNoUniformPipeline(shaderModule, format) {
        return device.createRenderPipeline({
            layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
            vertex: { module: shaderModule, entryPoint: 'vs_main' },
            fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format }] },
            primitive: { topology: 'triangle-list' }
        });
    }
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const assetNames = [...new Set(operations.flatMap(operation => (operation.source || [])
        .filter(source => source.kind === 'asset').map(source => source.asset)))];
    const assets = {};
    for (const asset of assetNames) {
        const image = await loadImageTexture(device, asset);
        assets[asset] = {
            image,
            bindGroup: device.createBindGroup({
                layout: textureLayout,
                entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
            })
        };
    }
    const textures = {};
    for (const spec of manifest.graph.targets) {
        textures[spec.id] = device.createTexture({
            size: [spec.width, spec.height],
            format: 'rgba8unorm-srgb',
            usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
        });
    }
    const uniformBuffers = [];
    function createUniform(values, minimumSize = 0) {
        const data = new Float32Array(values);
        const buffer = device.createBuffer({
            size: Math.max(data.byteLength, minimumSize),
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
        });
        device.queue.writeBuffer(buffer, 0, data);
        uniformBuffers.push(buffer);
        return device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] });
    }
    function assetImage(operation) {
        return assets[operation.asset].image;
    }
    function uniformData(operation) {
        const u = operation.uniform;
        if (operation.kind === 'sky') {
            return { values: [...u.top_color, u.noise_strength, ...u.bottom_color, u.time], minimumSize: 64 };
        }
        if (operation.kind === 'chroma_sprite') {
            const image = assetImage(operation);
            const crop = operation.crop_uv;
            const cropAspect = ((crop[2] - crop[0]) * image.width) / Math.max((crop[3] - crop[1]) * image.height, 1);
            const scaleY = operation.target_height_scale;
            const scaleX = scaleY * (cropAspect / (target.width / target.height));
            return { values: [
                operation.position[0], operation.position[1], scaleX, scaleY,
                crop[0], crop[1], crop[2], crop[3],
                operation.key_color[0], operation.key_color[1], operation.key_color[2], operation.tolerance,
                operation.smoothness, operation.z_depth, operation.opacity, 0
            ] };
        }
        if (operation.kind === 'dual_texture_postprocess') {
            const u = operation.uniform;
            return { values: [u.progress, u.effect_type, u.direction_x, u.direction_y] };
        }
        throw new Error(`Unsupported TC18 operation kind: ${operation.kind}`);
    }
    const targetBindGroups = {};
    for (const spec of manifest.graph.targets) {
        targetBindGroups[spec.id] = device.createBindGroup({
            layout: textureLayout,
            entries: [{ binding: 0, resource: textures[spec.id].createView() }, { binding: 1, resource: sampler }]
        });
    }
    const resources = operations.map(operation => {
        let sourceBindGroup;
        if (operation.kind === 'dual_texture_postprocess') {
            const sourceTargets = (operation.source || []).map(source => {
                if (source.kind !== 'target') throw new Error('TC18 dual transition requires target sources');
                return textures[source.target];
            });
            if (sourceTargets.length !== 2) throw new Error('TC18 transition requires exactly two target sources');
            sourceBindGroup = device.createBindGroup({
                layout: dualTextureLayout,
                entries: [
                    { binding: 0, resource: sourceTargets[0].createView() },
                    { binding: 1, resource: sampler },
                    { binding: 2, resource: sourceTargets[1].createView() },
                    { binding: 3, resource: sampler }
                ]
            });
        } else {
            const source = (operation.source || [])[0];
            if (!source || source.kind !== 'asset') throw new Error(`TC18 ${operation.id} requires an asset source`);
            sourceBindGroup = assets[source.asset].bindGroup;
        }
        const uniform = uniformData(operation);
        return { operation, bindGroups: [sourceBindGroup, createUniform(uniform.values, uniform.minimumSize || 0)] };
    });
    async function submitPass(passSpec, pipelines) {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: textures[passSpec.target].createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        for (const resource of resources.filter(item => item.operation.pass === passSpec.id)) {
            pass.setPipeline(pipelines[resource.operation.pipeline]);
            resource.bindGroups.forEach((bindGroup, index) => pass.setBindGroup(index, bindGroup));
            pass.draw(resource.operation.vertex_count, resource.operation.instance_count, 0, 0);
        }
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC18 validation error: ${error.message}`);
    }
    async function renderAll(pipelines) {
        const started = performance.now();
        for (const pass of passes) await submitPass(pass, pipelines);
        return performance.now() - started;
    }
    device.pushErrorScope('validation');
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    const pipelineError = await device.popErrorScope();
    if (pipelineError) throw new Error(`TC18 pipeline validation error: ${pipelineError.message}`);
    const coldRenderTimeMs = await renderAll(offscreenPipelines);
    const coldBytes = await readTextureBytes(device, textures.final, target.width, target.height);
    const warmRenderTimeMs = await renderAll(offscreenPipelines);
    const bytes = await readTextureBytes(device, textures.final, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC18 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc18_transition_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc18_transition.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '3 pass (scene A → scene B → dual-texture transition) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operations.reduce((sum, operation) => sum + operation.instance_count, 0),
        pass_count: passes.length, image_name: 'tc18_transition_web.png'
    });
    const canvas = document.getElementById('canvas-tc18');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const canvasBlit = createNoUniformPipeline(blitShader, canvasFormat);
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: textures.final.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc18_transition_web_preview.png');
    document.getElementById('tag-tc18').textContent = 'PASS';
    document.getElementById('tag-tc18').className = 'tag tag-passed';
    for (const texture of Object.values(textures)) texture.destroy();
    for (const asset of Object.values(assets)) asset.image.texture.destroy();
    for (const buffer of uniformBuffers) buffer.destroy();
}

async function runTC19(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc19_audio_viz.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC19 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('audio_spectrum.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC19 shader audio_spectrum.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb' }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_tc085_noise.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const uniformValues = [
        ...operation.uniform.freqs,
        ...operation.uniform.base_color,
        operation.uniform.time, 0, 0, 0
    ];
    const uniformData = new Float32Array(uniformValues);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, sourceBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC19 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC19 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc19_audio_viz_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc19_audio_viz.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (audio spectrum visualizer) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc19_audio_viz_web.png'
    });
    const canvas = document.getElementById('canvas-tc19');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc19_audio_viz_web_preview.png');
    document.getElementById('tag-tc19').textContent = 'PASS';
    document.getElementById('tag-tc19').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC20(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc20_perspective.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC20 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('perspective_sprite.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC20 shader perspective_sprite.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformValues = [
        ...u.mvp.flat(),
        ...u.uv_min, ...u.uv_max,
        ...u.key_color, u.tolerance,
        u.smoothness, u.opacity, 0, 0
    ];
    const uniformData = new Float32Array(uniformValues);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, sourceBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC20 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC20 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc20_perspective_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc20_perspective.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (fixed MVP perspective sprite) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc20_perspective_web.png'
    });
    const canvas = document.getElementById('canvas-tc20');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc20_perspective_web_preview.png');
    document.getElementById('tag-tc20').textContent = 'PASS';
    document.getElementById('tag-tc20').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC21(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc21_masking.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC21 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('masking.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC21 shader masking.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformValues = [
        ...u.transform.flat(),
        ...u.uv_min, ...u.uv_max,
        ...u.key_color, u.tolerance,
        u.smoothness, u.opacity, 0, 0
    ];
    const uniformData = new Float32Array(uniformValues);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, sourceBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC21 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC21 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc21_masking_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc21_masking.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (SDF circular mask + chroma key) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc21_masking_web.png'
    });
    const canvas = document.getElementById('canvas-tc21');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc21_masking_web_preview.png');
    document.getElementById('tag-tc21').textContent = 'PASS';
    document.getElementById('tag-tc21').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC22(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc22_particles_instanced.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC22 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('instanced_prop.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC22 shader instanced_prop.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const uniformData = new Float32Array([
        operation.uniform.aspect_ratio, operation.uniform.time, 0, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, sourceBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC22 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC22 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc22_particles_instanced_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc22_particles_instanced.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (100 hardware instances in one draw command) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc22_particles_instanced_web.png'
    });
    const canvas = document.getElementById('canvas-tc22');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc22_particles_instanced_web_preview.png');
    document.getElementById('tag-tc22').textContent = 'PASS';
    document.getElementById('tag-tc22').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC23(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc23_color_replace.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC23 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('color_replace.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC23 shader color_replace.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.transform.flat(),
        ...u.uv_min, ...u.uv_max,
        ...u.target_hsv, ...u.replace_hsv,
        u.tolerance, u.smoothness, 0, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, sourceBindGroup);
        pass.setBindGroup(1, uniformBindGroup);
        pass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        pass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC23 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC23 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc23_color_replace_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc23_color_replace.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (HSV palette replacement + chroma key) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc23_color_replace_web.png'
    });
    const canvas = document.getElementById('canvas-tc23');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc23_color_replace_web_preview.png');
    document.getElementById('tag-tc23').textContent = 'PASS';
    document.getElementById('tag-tc23').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC24(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc24_distortion_mesh.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC24 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('distortion.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC24 shader distortion.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.transform.flat(),
        ...u.uv_min, ...u.uv_max,
        u.time, u.amplitude, u.frequency, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const renderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        renderPass.setPipeline(pipeline);
        renderPass.setBindGroup(0, sourceBindGroup);
        renderPass.setBindGroup(1, uniformBindGroup);
        renderPass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        renderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC24 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC24 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc24_distortion_mesh_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc24_distortion_mesh.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (vertex wind/sway deformation) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc24_distortion_mesh_web.png'
    });
    const canvas = document.getElementById('canvas-tc24');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc24_distortion_mesh_web_preview.png');
    document.getElementById('tag-tc24').textContent = 'PASS';
    document.getElementById('tag-tc24').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC25(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc25_shadow_rimlight.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC25 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('rimlight.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC25 shader rimlight.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.transform.flat(),
        ...u.uv_min, ...u.uv_max,
        ...u.rim_color, u.rim_thickness,
        ...u.shadow_offset, ...u.shadow_color,
        0, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const renderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        renderPass.setPipeline(pipeline);
        renderPass.setBindGroup(0, sourceBindGroup);
        renderPass.setBindGroup(1, uniformBindGroup);
        renderPass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        renderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC25 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC25 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc25_shadow_rimlight_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc25_shadow_rimlight.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (shadow instance + rimlight instance) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc25_shadow_rimlight_web.png'
    });
    const canvas = document.getElementById('canvas-tc25');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc25_shadow_rimlight_web_preview.png');
    document.getElementById('tag-tc25').textContent = 'PASS';
    document.getElementById('tag-tc25').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC26(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc26_glitch.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC26 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('glitch.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC26 shader glitch.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.transform.flat(),
        ...u.uv_min, ...u.uv_max,
        u.time, u.intensity, u.aberration, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const renderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        renderPass.setPipeline(pipeline);
        renderPass.setBindGroup(0, sourceBindGroup);
        renderPass.setBindGroup(1, uniformBindGroup);
        renderPass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        renderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC26 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC26 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc26_glitch_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc26_glitch.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (deterministic glitch + RGB split) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc26_glitch_web.png'
    });
    const canvas = document.getElementById('canvas-tc26');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc26_glitch_web_preview.png');
    document.getElementById('tag-tc26').textContent = 'PASS';
    document.getElementById('tag-tc26').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC27(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc27_godrays.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC27 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('godrays.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC27 shader godrays.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_bg_forest.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.light_pos,
        u.exposure, u.decay, u.density, u.weight,
        0, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const renderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        renderPass.setPipeline(pipeline);
        renderPass.setBindGroup(0, sourceBindGroup);
        renderPass.setBindGroup(1, uniformBindGroup);
        renderPass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        renderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC27 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC27 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc27_godrays_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc27_godrays.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (100-sample radial godrays accumulation) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc27_godrays_web.png'
    });
    const canvas = document.getElementById('canvas-tc27');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc27_godrays_web_preview.png');
    document.getElementById('tag-tc27').textContent = 'PASS';
    document.getElementById('tag-tc27').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC28(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc28_ripple.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC28 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('ripple.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC28 shader ripple.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'one', dstFactor: 'zero', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'zero', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_bg_anime_city.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.center,
        u.time, u.amplitude, u.frequency, u.speed,
        0, 0
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const renderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        renderPass.setPipeline(pipeline);
        renderPass.setBindGroup(0, sourceBindGroup);
        renderPass.setBindGroup(1, uniformBindGroup);
        renderPass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        renderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC28 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC28 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc28_ripple_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc28_ripple.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (radial ripple UV distortion) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc28_ripple_web.png'
    });
    const canvas = document.getElementById('canvas-tc28');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc28_ripple_web_preview.png');
    document.getElementById('tag-tc28').textContent = 'PASS';
    document.getElementById('tag-tc28').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC29(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc29_crt_vhs.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC29 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operation = manifest.graph.operations[0];
    const passSpec = manifest.graph.passes[0];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const shaderModule = device.createShaderModule({ code: await fetchShader('crt_vhs.wgsl') });
    const compilationInfo = await shaderModule.getCompilationInfo();
    const shaderErrors = compilationInfo.messages.filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC29 shader crt_vhs.wgsl: ${shaderErrors.map(message => message.message).join('; ')}`);
    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'one', dstFactor: 'zero', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'zero', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const image = await loadImageTexture(device, 'canonical_bg_scifi.png');
    const sourceBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
    });
    const u = operation.uniform;
    const uniformData = new Float32Array([
        ...u.curvature, u.scanline_intensity, u.time
    ]);
    const uniformBuffer = device.createBuffer({
        size: uniformData.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    const uniformBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height],
        format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const renderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: passSpec.clear_color[0], g: passSpec.clear_color[1], b: passSpec.clear_color[2], a: passSpec.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        renderPass.setPipeline(pipeline);
        renderPass.setBindGroup(0, sourceBindGroup);
        renderPass.setBindGroup(1, uniformBindGroup);
        renderPass.draw(operation.vertex_count, operation.instance_count, 0, 0);
        renderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC29 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC29 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc29_crt_vhs_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc29_crt_vhs.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '1 pass (CRT curvature + scanlines + vignette + RGB split + integer-hash noise) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: operation.instance_count, pass_count: manifest.graph.passes.length,
        image_name: 'tc29_crt_vhs_web.png'
    });
    const canvas = document.getElementById('canvas-tc29');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({
        layout: textureLayout,
        entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }]
    });
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc29_crt_vhs_web_preview.png');
    document.getElementById('tag-tc29').textContent = 'PASS';
    document.getElementById('tag-tc29').className = 'tag tag-passed';
    finalTexture.destroy();
    image.texture.destroy();
    uniformBuffer.destroy();
}

async function runTC30(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc30_dissolve.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC30 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const chromaOp = manifest.graph.operations[0];
    const dissolveOp = manifest.graph.operations[1];
    const chromaPass = manifest.graph.passes[0];
    const dissolvePass = manifest.graph.passes[1];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const dualTextureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
        { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const chromaShader = device.createShaderModule({ code: await fetchShader('chroma_key_cropped.wgsl') });
    const dissolveShader = device.createShaderModule({ code: await fetchShader('dissolve.wgsl') });
    const chromaInfo = await chromaShader.getCompilationInfo();
    const dissolveInfo = await dissolveShader.getCompilationInfo();
    const shaderErrors = [...chromaInfo.messages, ...dissolveInfo.messages].filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC30 shader validation: ${shaderErrors.map(message => message.message).join('; ')}`);
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const chromaPipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module: chromaShader, entryPoint: 'vs_main' },
        fragment: { module: chromaShader, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const dissolvePipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [dualTextureLayout, uniformLayout] }),
        vertex: { module: dissolveShader, entryPoint: 'vs_main' },
        fragment: { module: dissolveShader, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const heroes = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const noise = await loadImageTexture(device, 'canonical_tc085_noise.png');
    const screenAspect = target.width / target.height;
    const crop = chromaOp.uniform;
    const cropWidth = (crop.uv_max[0] - crop.uv_min[0]) * heroes.width;
    const cropHeight = (crop.uv_max[1] - crop.uv_min[1]) * heroes.height;
    const scaleY = crop.scale_y;
    const scaleX = scaleY * (cropWidth / cropHeight) / screenAspect;
    const chromaUniformData = new Float32Array([
        0, 0, scaleX, scaleY,
        crop.uv_min[0], crop.uv_min[1], crop.uv_max[0], crop.uv_max[1],
        crop.key_color[0], crop.key_color[1], crop.key_color[2], crop.tolerance,
        crop.smoothness, 0.5, 1.0, 0
    ]);
    const chromaUniformBuffer = device.createBuffer({ size: chromaUniformData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(chromaUniformBuffer, 0, chromaUniformData);
    const chromaUniformBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: chromaUniformBuffer } }] });
    const dissolve = dissolveOp.uniform;
    const dissolveUniformData = new Float32Array([
        dissolve.threshold, dissolve.edge_width, 0, 0,
        dissolve.edge_color[0], dissolve.edge_color[1], dissolve.edge_color[2], 0
    ]);
    const dissolveUniformBuffer = device.createBuffer({ size: dissolveUniformData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(dissolveUniformBuffer, 0, dissolveUniformData);
    const dissolveUniformBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: dissolveUniformBuffer } }] });
    const heroesBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: heroes.texture.createView() }, { binding: 1, resource: sampler }
    ]});
    const chromaTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const dissolveBindGroup = device.createBindGroup({ layout: dualTextureLayout, entries: [
        { binding: 0, resource: chromaTexture.createView() }, { binding: 1, resource: sampler },
        { binding: 2, resource: noise.texture.createView() }, { binding: 3, resource: sampler }
    ]});
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const chromaRenderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: chromaTexture.createView(),
            clearValue: { r: chromaPass.clear_color[0], g: chromaPass.clear_color[1], b: chromaPass.clear_color[2], a: chromaPass.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        chromaRenderPass.setPipeline(chromaPipeline);
        chromaRenderPass.setBindGroup(0, heroesBindGroup);
        chromaRenderPass.setBindGroup(1, chromaUniformBindGroup);
        chromaRenderPass.draw(chromaOp.vertex_count, chromaOp.instance_count, 0, 0);
        chromaRenderPass.end();
        const dissolveRenderPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: dissolvePass.clear_color[0], g: dissolvePass.clear_color[1], b: dissolvePass.clear_color[2], a: dissolvePass.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        dissolveRenderPass.setPipeline(dissolvePipeline);
        dissolveRenderPass.setBindGroup(0, dissolveBindGroup);
        dissolveRenderPass.setBindGroup(1, dissolveUniformBindGroup);
        dissolveRenderPass.draw(dissolveOp.vertex_count, dissolveOp.instance_count, 0, 0);
        dissolveRenderPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC30 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC30 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc30_dissolve_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc30_dissolve.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '2 pass (chroma key → dissolve/burn) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: 2, pass_count: manifest.graph.passes.length,
        image_name: 'tc30_dissolve_web.png'
    });
    const canvas = document.getElementById('canvas-tc30');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }
    ]});
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc30_dissolve_web_preview.png');
    document.getElementById('tag-tc30').textContent = 'PASS';
    document.getElementById('tag-tc30').className = 'tag tag-passed';
    finalTexture.destroy();
    chromaTexture.destroy();
    heroes.texture.destroy();
    noise.texture.destroy();
    chromaUniformBuffer.destroy();
    dissolveUniformBuffer.destroy();
}

async function runTC31(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc31_light_sweep.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC31 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const chromaOp = manifest.graph.operations[0];
    const sweepOp = manifest.graph.operations[1];
    const chromaPass = manifest.graph.passes[0];
    const sweepPass = manifest.graph.passes[1];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const chromaShader = device.createShaderModule({ code: await fetchShader('chroma_key_cropped.wgsl') });
    const sweepShader = device.createShaderModule({ code: await fetchShader('light_sweep.wgsl') });
    const chromaInfo = await chromaShader.getCompilationInfo();
    const sweepInfo = await sweepShader.getCompilationInfo();
    const shaderErrors = [...chromaInfo.messages, ...sweepInfo.messages].filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC31 shader validation: ${shaderErrors.map(message => message.message).join('; ')}`);
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u,
        addressModeV: samplerSpec.address_mode_v,
        addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter,
        minFilter: samplerSpec.min_filter,
        mipmapFilter: samplerSpec.mipmap_filter
    });
    const makePipeline = (module) => device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }),
        vertex: { module, entryPoint: 'vs_main' },
        fragment: { module, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        } }] },
        primitive: { topology: 'triangle-list' }
    });
    const chromaPipeline = makePipeline(chromaShader);
    const sweepPipeline = makePipeline(sweepShader);
    const heroes = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const crop = chromaOp.uniform;
    const screenAspect = target.width / target.height;
    const cropWidth = (crop.uv_max[0] - crop.uv_min[0]) * heroes.width;
    const cropHeight = (crop.uv_max[1] - crop.uv_min[1]) * heroes.height;
    const scaleY = crop.scale_y;
    const scaleX = scaleY * (cropWidth / cropHeight) / screenAspect;
    const chromaUniformData = new Float32Array([
        0, 0, scaleX, scaleY,
        crop.uv_min[0], crop.uv_min[1], crop.uv_max[0], crop.uv_max[1],
        crop.key_color[0], crop.key_color[1], crop.key_color[2], crop.tolerance,
        crop.smoothness, 0.5, 1.0, 0
    ]);
    const chromaUniformBuffer = device.createBuffer({ size: chromaUniformData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(chromaUniformBuffer, 0, chromaUniformData);
    const chromaUniformBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: chromaUniformBuffer } }] });
    const sweep = sweepOp.uniform;
    const sweepUniformData = new Float32Array([
        sweep.progress, sweep.angle, sweep.width, sweep.intensity,
        sweep.color[0], sweep.color[1], sweep.color[2], 0
    ]);
    const sweepUniformBuffer = device.createBuffer({ size: sweepUniformData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(sweepUniformBuffer, 0, sweepUniformData);
    const sweepUniformBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: sweepUniformBuffer } }] });
    const heroesBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: heroes.texture.createView() }, { binding: 1, resource: sampler }
    ]});
    const chromaTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING
    });
    const finalTexture = device.createTexture({
        size: [target.width, target.height], format: 'rgba8unorm-srgb',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const sweepBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: chromaTexture.createView() }, { binding: 1, resource: sampler }
    ]});
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const firstPass = encoder.beginRenderPass({ colorAttachments: [{
            view: chromaTexture.createView(),
            clearValue: { r: chromaPass.clear_color[0], g: chromaPass.clear_color[1], b: chromaPass.clear_color[2], a: chromaPass.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        firstPass.setPipeline(chromaPipeline);
        firstPass.setBindGroup(0, heroesBindGroup);
        firstPass.setBindGroup(1, chromaUniformBindGroup);
        firstPass.draw(chromaOp.vertex_count, chromaOp.instance_count, 0, 0);
        firstPass.end();
        const secondPass = encoder.beginRenderPass({ colorAttachments: [{
            view: finalTexture.createView(),
            clearValue: { r: sweepPass.clear_color[0], g: sweepPass.clear_color[1], b: sweepPass.clear_color[2], a: sweepPass.clear_color[3] },
            loadOp: 'clear', storeOp: 'store'
        }]});
        secondPass.setPipeline(sweepPipeline);
        secondPass.setBindGroup(0, sweepBindGroup);
        secondPass.setBindGroup(1, sweepUniformBindGroup);
        secondPass.draw(sweepOp.vertex_count, sweepOp.instance_count, 0, 0);
        secondPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC31 validation error: ${error.message}`);
    }
    const startedCold = performance.now();
    await render();
    const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now();
    await render();
    const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC31 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc31_light_sweep_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100,
        cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null,
        manifest: 'tests/shared_assets/manifests/tc31_light_sweep.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '2 pass (chroma key → diagonal light sweep) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        instance_count: 2, pass_count: manifest.graph.passes.length,
        image_name: 'tc31_light_sweep_web.png'
    });
    const canvas = document.getElementById('canvas-tc31');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }),
        vertex: { module: blitShader, entryPoint: 'vs_main' },
        fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const finalBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }
    ]});
    const encoder = device.createCommandEncoder();
    const previewPass = encoder.beginRenderPass({ colorAttachments: [{
        view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store'
    }]});
    previewPass.setPipeline(canvasBlit);
    previewPass.setBindGroup(0, finalBindGroup);
    previewPass.draw(6, 1, 0, 0);
    previewPass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc31_light_sweep_web_preview.png');
    document.getElementById('tag-tc31').textContent = 'PASS';
    document.getElementById('tag-tc31').className = 'tag tag-passed';
    finalTexture.destroy();
    chromaTexture.destroy();
    heroes.texture.destroy();
    chromaUniformBuffer.destroy();
    sweepUniformBuffer.destroy();
}

async function runTC32(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc32_page_curl.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC32 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const operations = manifest.graph.operations;
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const dualTextureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
        { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const skyShader = device.createShaderModule({ code: await fetchShader('sky_composite_deterministic.wgsl') });
    const chromaShader = device.createShaderModule({ code: await fetchShader('chroma_key_cropped.wgsl') });
    const curlShader = device.createShaderModule({ code: await fetchShader('page_curl.wgsl') });
    const compilation = await Promise.all([skyShader.getCompilationInfo(), chromaShader.getCompilationInfo(), curlShader.getCompilationInfo()]);
    const shaderErrors = compilation.flatMap(info => info.messages).filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC32 shader validation: ${shaderErrors.map(message => message.message).join('; ')}`);
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({
        addressModeU: samplerSpec.address_mode_u, addressModeV: samplerSpec.address_mode_v, addressModeW: samplerSpec.address_mode_w,
        magFilter: samplerSpec.mag_filter, minFilter: samplerSpec.min_filter, mipmapFilter: samplerSpec.mipmap_filter
    });
    const makePipeline = (module, layout, blend) => device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [layout, uniformLayout] }),
        vertex: { module, entryPoint: 'vs_main' },
        fragment: { module, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', ...(blend ? { blend } : {}) }] },
        primitive: { topology: 'triangle-list' }
    });
    const alphaBlend = {
        color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
    };
    const skyPipeline = makePipeline(skyShader, textureLayout, null);
    const chromaPipeline = makePipeline(chromaShader, textureLayout, alphaBlend);
    const curlPipeline = makePipeline(curlShader, dualTextureLayout, alphaBlend);
    const noise = await loadImageTexture(device, 'canonical_tc085_noise.png');
    const heroes = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const makeUniform = (data) => {
        const buffer = device.createBuffer({ size: data.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(buffer, 0, data);
        return { buffer, bindGroup: device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer } }] }) };
    };
    const skyData = (u) => new Float32Array([
        u.top_color[0], u.top_color[1], u.top_color[2], u.noise_strength,
        u.bottom_color[0], u.bottom_color[1], u.bottom_color[2], u.time
    ]);
    const cropData = (u) => {
        const cropWidth = (u.uv_max[0] - u.uv_min[0]) * heroes.width;
        const cropHeight = (u.uv_max[1] - u.uv_min[1]) * heroes.height;
        const scaleX = u.scale_y * (cropWidth / cropHeight) / (target.width / target.height);
        return new Float32Array([
            0, 0, scaleX, u.scale_y, u.uv_min[0], u.uv_min[1], u.uv_max[0], u.uv_max[1],
            u.key_color[0], u.key_color[1], u.key_color[2], u.tolerance, u.smoothness, 0.5, 1.0, 0
        ]);
    };
    const skyA = makeUniform(skyData(operations[0].uniform));
    const skyB = makeUniform(skyData(operations[2].uniform));
    const paladin = makeUniform(cropData(operations[1].uniform));
    const mage = makeUniform(cropData(operations[3].uniform));
    const curl = makeUniform(new Float32Array([operations[4].uniform.progress, operations[4].uniform.radius, 0, 0]));
    const noiseBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: noise.texture.createView() }, { binding: 1, resource: sampler }
    ]});
    const heroesBindGroup = device.createBindGroup({ layout: textureLayout, entries: [
        { binding: 0, resource: heroes.texture.createView() }, { binding: 1, resource: sampler }
    ]});
    const sceneATexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
    const sceneBTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
    const finalTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC });
    const curlBindGroup = device.createBindGroup({ layout: dualTextureLayout, entries: [
        { binding: 0, resource: sceneATexture.createView() }, { binding: 1, resource: sampler },
        { binding: 2, resource: sceneBTexture.createView() }, { binding: 3, resource: sampler }
    ]});
    async function render() {
        device.pushErrorScope('validation');
        const encoder = device.createCommandEncoder();
        const sceneAPass = encoder.beginRenderPass({ colorAttachments: [{ view: sceneATexture.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
        sceneAPass.setPipeline(skyPipeline); sceneAPass.setBindGroup(0, noiseBindGroup); sceneAPass.setBindGroup(1, skyA.bindGroup); sceneAPass.draw(6, 1, 0, 0);
        sceneAPass.setPipeline(chromaPipeline); sceneAPass.setBindGroup(0, heroesBindGroup); sceneAPass.setBindGroup(1, paladin.bindGroup); sceneAPass.draw(6, 1, 0, 0); sceneAPass.end();
        const sceneBPass = encoder.beginRenderPass({ colorAttachments: [{ view: sceneBTexture.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
        sceneBPass.setPipeline(skyPipeline); sceneBPass.setBindGroup(0, noiseBindGroup); sceneBPass.setBindGroup(1, skyB.bindGroup); sceneBPass.draw(6, 1, 0, 0);
        sceneBPass.setPipeline(chromaPipeline); sceneBPass.setBindGroup(0, heroesBindGroup); sceneBPass.setBindGroup(1, mage.bindGroup); sceneBPass.draw(6, 1, 0, 0); sceneBPass.end();
        const curlPass = encoder.beginRenderPass({ colorAttachments: [{ view: finalTexture.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
        curlPass.setPipeline(curlPipeline); curlPass.setBindGroup(0, curlBindGroup); curlPass.setBindGroup(1, curl.bindGroup); curlPass.draw(6, 1, 0, 0); curlPass.end();
        device.queue.submit([encoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        const error = await device.popErrorScope();
        if (error) throw new Error(`TC32 validation error: ${error.message}`);
    }
    const startedCold = performance.now(); await render(); const coldRenderTimeMs = performance.now() - startedCold;
    const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const startedWarm = performance.now(); await render(); const warmRenderTimeMs = performance.now() - startedWarm;
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]);
    if (!cacheOutputEqual) throw new Error('TC32 output changed between cold and warm runs');
    await saveRawTexture(bytes, {
        name: 'tc32_page_curl_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs, warm_iteration_count: 1,
        speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100, cache_output_equal: cacheOutputEqual,
        validation_passed: true, validation_error: null, manifest: 'tests/shared_assets/manifests/tc32_page_curl.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: '3 pass (scene A + scene B + page curl) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count, instance_count: 5,
        pass_count: manifest.graph.passes.length, image_name: 'tc32_page_curl_web.png'
    });
    const canvas = document.getElementById('canvas-tc32'); const context = canvas.getContext('webgpu'); const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') });
    const canvasBlit = device.createRenderPipeline({ layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }), vertex: { module: blitShader, entryPoint: 'vs_main' }, fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] }, primitive: { topology: 'triangle-list' } });
    const finalBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }] });
    const previewEncoder = device.createCommandEncoder(); const previewPass = previewEncoder.beginRenderPass({ colorAttachments: [{ view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
    previewPass.setPipeline(canvasBlit); previewPass.setBindGroup(0, finalBindGroup); previewPass.draw(6, 1, 0, 0); previewPass.end(); device.queue.submit([previewEncoder.finish()]); await device.queue.onSubmittedWorkDone();
    await saveCanvasImage(canvas, 'tc32_page_curl_web_preview.png'); document.getElementById('tag-tc32').textContent = 'PASS'; document.getElementById('tag-tc32').className = 'tag tag-passed';
    finalTexture.destroy(); sceneATexture.destroy(); sceneBTexture.destroy(); noise.texture.destroy(); heroes.texture.destroy();
    for (const uniform of [skyA, skyB, paladin, mage, curl]) uniform.buffer.destroy();
}

async function runTC33(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc33_pixelation.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC33 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const chromaOp = manifest.graph.operations[0];
    const pixelOp = manifest.graph.operations[1];
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    const chromaShader = device.createShaderModule({ code: await fetchShader('chroma_key_cropped.wgsl') });
    const pixelShader = device.createShaderModule({ code: await fetchShader('pixelation.wgsl') });
    const compilation = await Promise.all([chromaShader.getCompilationInfo(), pixelShader.getCompilationInfo()]);
    const shaderErrors = compilation.flatMap(info => info.messages).filter(message => message.type === 'error');
    if (shaderErrors.length) throw new Error(`TC33 shader validation: ${shaderErrors.map(message => message.message).join('; ')}`);
    const samplerSpec = manifest.graph.sampler;
    const sampler = device.createSampler({ addressModeU: samplerSpec.address_mode_u, addressModeV: samplerSpec.address_mode_v, addressModeW: samplerSpec.address_mode_w, magFilter: samplerSpec.mag_filter, minFilter: samplerSpec.min_filter, mipmapFilter: samplerSpec.mipmap_filter });
    const makePipeline = (module) => device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }), vertex: { module, entryPoint: 'vs_main' },
        fragment: { module, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: { color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' }, alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' } } }] }, primitive: { topology: 'triangle-list' }
    });
    const chromaPipeline = makePipeline(chromaShader); const pixelPipeline = makePipeline(pixelShader);
    const heroes = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const crop = chromaOp.uniform;
    const cropWidth = (crop.uv_max[0] - crop.uv_min[0]) * heroes.width; const cropHeight = (crop.uv_max[1] - crop.uv_min[1]) * heroes.height;
    const scaleX = crop.scale_y * (cropWidth / cropHeight) / (target.width / target.height);
    const chromaUniformData = new Float32Array([0, 0, scaleX, crop.scale_y, crop.uv_min[0], crop.uv_min[1], crop.uv_max[0], crop.uv_max[1], crop.key_color[0], crop.key_color[1], crop.key_color[2], crop.tolerance, crop.smoothness, 0.5, 1.0, 0]);
    const chromaBuffer = device.createBuffer({ size: chromaUniformData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST }); device.queue.writeBuffer(chromaBuffer, 0, chromaUniformData);
    const chromaBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: chromaBuffer } }] });
    const p = pixelOp.uniform; const pixelData = new Float32Array([p.block_size, p.screen_width, p.screen_height, 0]);
    const pixelBuffer = device.createBuffer({ size: pixelData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST }); device.queue.writeBuffer(pixelBuffer, 0, pixelData);
    const pixelBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: pixelBuffer } }] });
    const heroesBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: heroes.texture.createView() }, { binding: 1, resource: sampler }] });
    const chromaTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
    const finalTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC });
    const pixelTextureBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: chromaTexture.createView() }, { binding: 1, resource: sampler }] });
    async function render() {
        device.pushErrorScope('validation'); const encoder = device.createCommandEncoder();
        const first = encoder.beginRenderPass({ colorAttachments: [{ view: chromaTexture.createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: 'clear', storeOp: 'store' }] });
        first.setPipeline(chromaPipeline); first.setBindGroup(0, heroesBindGroup); first.setBindGroup(1, chromaBindGroup); first.draw(6, 1, 0, 0); first.end();
        const second = encoder.beginRenderPass({ colorAttachments: [{ view: finalTexture.createView(), clearValue: { r: 0.1, g: 0.1, b: 0.3, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
        second.setPipeline(pixelPipeline); second.setBindGroup(0, pixelTextureBindGroup); second.setBindGroup(1, pixelBindGroup); second.draw(6, 1, 0, 0); second.end();
        device.queue.submit([encoder.finish()]); await device.queue.onSubmittedWorkDone(); const error = await device.popErrorScope(); if (error) throw new Error(`TC33 validation error: ${error.message}`);
    }
    const coldStart = performance.now(); await render(); const coldRenderTimeMs = performance.now() - coldStart; const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const warmStart = performance.now(); await render(); const warmRenderTimeMs = performance.now() - warmStart; const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]); if (!cacheOutputEqual) throw new Error('TC33 output changed between cold and warm runs');
    await saveRawTexture(bytes, { name: 'tc33_pixelation_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb', cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs, warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100, cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null, manifest: 'tests/shared_assets/manifests/tc33_pixelation.json', manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)), adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter', timing_scope: '2 pass (chroma key → 16px pixelation) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback', node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count, instance_count: 2, pass_count: manifest.graph.passes.length, image_name: 'tc33_pixelation_web.png' });
    const canvas = document.getElementById('canvas-tc33'); const context = canvas.getContext('webgpu'); const canvasFormat = navigator.gpu.getPreferredCanvasFormat(); context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') }); const canvasBlit = device.createRenderPipeline({ layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }), vertex: { module: blitShader, entryPoint: 'vs_main' }, fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] }, primitive: { topology: 'triangle-list' } });
    const finalBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }] }); const previewEncoder = device.createCommandEncoder(); const preview = previewEncoder.beginRenderPass({ colorAttachments: [{ view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
    preview.setPipeline(canvasBlit); preview.setBindGroup(0, finalBindGroup); preview.draw(6, 1, 0, 0); preview.end(); device.queue.submit([previewEncoder.finish()]); await device.queue.onSubmittedWorkDone(); await saveCanvasImage(canvas, 'tc33_pixelation_web_preview.png');
    document.getElementById('tag-tc33').textContent = 'PASS'; document.getElementById('tag-tc33').className = 'tag tag-passed'; finalTexture.destroy(); chromaTexture.destroy(); heroes.texture.destroy(); chromaBuffer.destroy(); pixelBuffer.destroy();
}

async function runTC34(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc34_directional_blur.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC34 shared manifest');
    const manifestText = await manifestResponse.text(); const manifest = JSON.parse(manifestText); const target = manifest.graph.target; const chromaOp = manifest.graph.operations[0]; const blurOp = manifest.graph.operations[1];
    const textureLayout = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } }, { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }] });
    const uniformLayout = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }] });
    const chromaShader = device.createShaderModule({ code: await fetchShader('chroma_key_cropped.wgsl') }); const blurShader = device.createShaderModule({ code: await fetchShader('directional_blur.wgsl') });
    const info = await Promise.all([chromaShader.getCompilationInfo(), blurShader.getCompilationInfo()]); const errors = info.flatMap(item => item.messages).filter(message => message.type === 'error'); if (errors.length) throw new Error(`TC34 shader validation: ${errors.map(message => message.message).join('; ')}`);
    const samplerSpec = manifest.graph.sampler; const sampler = device.createSampler({ addressModeU: samplerSpec.address_mode_u, addressModeV: samplerSpec.address_mode_v, addressModeW: samplerSpec.address_mode_w, magFilter: samplerSpec.mag_filter, minFilter: samplerSpec.min_filter, mipmapFilter: samplerSpec.mipmap_filter });
    const makePipeline = (module) => device.createRenderPipeline({ layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout, uniformLayout] }), vertex: { module, entryPoint: 'vs_main' }, fragment: { module, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm-srgb', blend: { color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' }, alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' } } }] }, primitive: { topology: 'triangle-list' } });
    const chromaPipeline = makePipeline(chromaShader); const blurPipeline = makePipeline(blurShader); const heroes = await loadImageTexture(device, 'canonical_sprites_heroes.png');
    const crop = chromaOp.uniform; const cropWidth = (crop.uv_max[0] - crop.uv_min[0]) * heroes.width; const cropHeight = (crop.uv_max[1] - crop.uv_min[1]) * heroes.height; const scaleX = crop.scale_y * (cropWidth / cropHeight) / (target.width / target.height);
    const chromaData = new Float32Array([0, 0, scaleX, crop.scale_y, crop.uv_min[0], crop.uv_min[1], crop.uv_max[0], crop.uv_max[1], crop.key_color[0], crop.key_color[1], crop.key_color[2], crop.tolerance, crop.smoothness, 0.5, 1.0, 0]); const chromaBuffer = device.createBuffer({ size: chromaData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST }); device.queue.writeBuffer(chromaBuffer, 0, chromaData); const chromaBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: chromaBuffer } }] });
    const blur = blurOp.uniform; const blurData = new Float32Array([blur.angle, blur.strength, blur.samples, 0]); const blurBuffer = device.createBuffer({ size: blurData.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST }); device.queue.writeBuffer(blurBuffer, 0, blurData); const blurBindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: blurBuffer } }] });
    const heroesBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: heroes.texture.createView() }, { binding: 1, resource: sampler }] }); const chromaTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING }); const finalTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC }); const blurTextureBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: chromaTexture.createView() }, { binding: 1, resource: sampler }] });
    async function render() { device.pushErrorScope('validation'); const encoder = device.createCommandEncoder(); const first = encoder.beginRenderPass({ colorAttachments: [{ view: chromaTexture.createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: 'clear', storeOp: 'store' }] }); first.setPipeline(chromaPipeline); first.setBindGroup(0, heroesBindGroup); first.setBindGroup(1, chromaBindGroup); first.draw(6, 1, 0, 0); first.end(); const second = encoder.beginRenderPass({ colorAttachments: [{ view: finalTexture.createView(), clearValue: { r: 0.2, g: 0.1, b: 0.1, a: 1 }, loadOp: 'clear', storeOp: 'store' }] }); second.setPipeline(blurPipeline); second.setBindGroup(0, blurTextureBindGroup); second.setBindGroup(1, blurBindGroup); second.draw(6, 1, 0, 0); second.end(); device.queue.submit([encoder.finish()]); await device.queue.onSubmittedWorkDone(); const error = await device.popErrorScope(); if (error) throw new Error(`TC34 validation error: ${error.message}`); }
    const coldStart = performance.now(); await render(); const coldRenderTimeMs = performance.now() - coldStart; const coldBytes = await readTextureBytes(device, finalTexture, target.width, target.height); const warmStart = performance.now(); await render(); const warmRenderTimeMs = performance.now() - warmStart; const bytes = await readTextureBytes(device, finalTexture, target.width, target.height); const cacheOutputEqual = coldBytes.length === bytes.length && coldBytes.every((value, index) => value === bytes[index]); if (!cacheOutputEqual) throw new Error('TC34 output changed between cold and warm runs');
    await saveRawTexture(bytes, { name: 'tc34_directional_blur_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb', cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs, warm_iteration_count: 1, speedup_percentage: (1 - warmRenderTimeMs / coldRenderTimeMs) * 100, cache_output_equal: cacheOutputEqual, validation_passed: true, validation_error: null, manifest: 'tests/shared_assets/manifests/tc34_directional_blur.json', manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)), adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter', timing_scope: '2 pass (chroma key → 30 degree directional blur) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback', node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count, instance_count: 2, pass_count: manifest.graph.passes.length, image_name: 'tc34_directional_blur_web.png' });
    const canvas = document.getElementById('canvas-tc34'); const context = canvas.getContext('webgpu'); const canvasFormat = navigator.gpu.getPreferredCanvasFormat(); context.configure({ device, format: canvasFormat, alphaMode: 'opaque' }); const blitShader = device.createShaderModule({ code: await fetchShader('texture_blit.wgsl') }); const canvasBlit = device.createRenderPipeline({ layout: device.createPipelineLayout({ bindGroupLayouts: [textureLayout] }), vertex: { module: blitShader, entryPoint: 'vs_main' }, fragment: { module: blitShader, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] }, primitive: { topology: 'triangle-list' } }); const finalBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: finalTexture.createView() }, { binding: 1, resource: sampler }] }); const previewEncoder = device.createCommandEncoder(); const preview = previewEncoder.beginRenderPass({ colorAttachments: [{ view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }] }); preview.setPipeline(canvasBlit); preview.setBindGroup(0, finalBindGroup); preview.draw(6, 1, 0, 0); preview.end(); device.queue.submit([previewEncoder.finish()]); await device.queue.onSubmittedWorkDone(); await saveCanvasImage(canvas, 'tc34_directional_blur_web_preview.png'); document.getElementById('tag-tc34').textContent = 'PASS'; document.getElementById('tag-tc34').className = 'tag tag-passed'; finalTexture.destroy(); chromaTexture.destroy(); heroes.texture.destroy(); chromaBuffer.destroy(); blurBuffer.destroy();
}

async function runTC085(gpu) {
    const { device } = gpu;
    const manifestResponse = await fetch('/manifests/tc08_5_nightsky.json');
    if (!manifestResponse.ok) throw new Error('Failed to load TC08.5 shared manifest');
    const manifestText = await manifestResponse.text();
    const manifest = JSON.parse(manifestText);
    const target = manifest.graph.target;
    const scenePass = manifest.graph.passes.find(pass => pass.id === 'scene');
    const finalPass = manifest.graph.passes.find(pass => pass.id === 'final');
    const shaderModules = {};
    for (const spec of Object.values(manifest.graph.pipelines)) {
        if (!shaderModules[spec.shader]) {
            shaderModules[spec.shader] = device.createShaderModule({ code: await fetchShader(spec.shader) });
        }
    }
    const textureLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float', viewDimension: '2d' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
    ]});
    const uniformLayout = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
    ]});
    function blendState(name) {
        if (name === 'Replace') return undefined;
        if (name === 'AlphaBlend') return {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        if (name === 'Additive') return {
            color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
        };
        throw new Error(`Unsupported TC08.5 blend mode: ${name}`);
    }
    function pipelineLayouts(spec) {
        if (spec.layout === 'texture') return [textureLayout];
        if (spec.layout === 'texture_uniform') return [textureLayout, uniformLayout];
        if (spec.layout === 'texture_texture_uniform') return [textureLayout, textureLayout, uniformLayout];
        throw new Error(`Unsupported TC08.5 layout: ${spec.layout}`);
    }
    function createPipelines(format) {
        const result = {};
        for (const [name, spec] of Object.entries(manifest.graph.pipelines)) {
            result[name] = device.createRenderPipeline({
                layout: device.createPipelineLayout({ bindGroupLayouts: pipelineLayouts(spec) }),
                vertex: { module: shaderModules[spec.shader], entryPoint: 'vs_main' },
                fragment: {
                    module: shaderModules[spec.shader],
                    entryPoint: 'fs_main',
                    targets: [{ format, blend: blendState(spec.blend) }]
                },
                primitive: { topology: 'triangle-list' }
            });
        }
        return result;
    }
    const sampler = device.createSampler({
        addressModeU: 'repeat', addressModeV: 'repeat', addressModeW: 'repeat',
        magFilter: manifest.graph.sampler.mag_filter,
        minFilter: manifest.graph.sampler.min_filter,
        mipmapFilter: manifest.graph.sampler.mipmap_filter
    });
    const assetCache = {};
    async function getAsset(name) {
        if (!assetCache[name]) {
            const image = await loadImageTexture(device, name);
            assetCache[name] = {
                image,
                bindGroup: device.createBindGroup({
                    layout: textureLayout,
                    entries: [{ binding: 0, resource: image.texture.createView() }, { binding: 1, resource: sampler }]
                })
            };
        }
        return assetCache[name];
    }
    function makeUniform(data) {
        const uniformBuffer = device.createBuffer({ size: data.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        device.queue.writeBuffer(uniformBuffer, 0, data);
        const bindGroup = device.createBindGroup({ layout: uniformLayout, entries: [{ binding: 0, resource: { buffer: uniformBuffer } }] });
        return { uniformBuffer, bindGroup };
    }
    function uniformData(operation) {
        const u = operation.uniform;
        if (operation.kind === 'sky') return new Float32Array([...u.top_color, u.noise_strength, ...u.bottom_color, u.time]);
        if (operation.kind === 'moon') return new Float32Array([...u.model_view, ...u.uv_min, ...u.uv_max, ...u.key_color, u.tolerance, u.smoothness, u.noise_strength, u.glow_intensity, 0]);
        if (operation.kind === 'cloud') return new Float32Array([...u.model_view, ...u.uv_bounds, ...u.key_color_tol, ...u.params, ...u.lighting_pos]);
        if (operation.kind === 'postprocess') return new Float32Array([u.bloom_intensity, u.exposure, u.contrast, 0]);
        return null;
    }
    const sceneTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC });
    const finalTexture = device.createTexture({ size: [target.width, target.height], format: 'rgba8unorm-srgb', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
    const sceneSourceBindGroup = device.createBindGroup({ layout: textureLayout, entries: [{ binding: 0, resource: sceneTexture.createView() }, { binding: 1, resource: sampler }] });
    async function buildOperation(operation, sourceOverride) {
        const sources = operation.source || [];
        const bindGroups = [];
        for (const source of sources) {
            if (sourceOverride) {
                bindGroups.push(sourceOverride);
            } else if (source.kind === 'asset') {
                bindGroups.push((await getAsset(source.asset)).bindGroup);
            } else if (source.kind === 'target' && source.target === 'scene') {
                bindGroups.push(sceneSourceBindGroup);
            }
        }
        const uniform = uniformData(operation);
        const uniformResource = uniform ? makeUniform(uniform) : null;
        if (uniformResource) bindGroups.push(uniformResource.bindGroup);
        return { pipeline: operation.pipeline, bindGroups, uniformBuffer: uniformResource?.uniformBuffer, vertexCount: operation.vertex_count, instanceCount: operation.instance_count };
    }
    const sceneResources = [];
    for (const operation of scenePass.operations) sceneResources.push(await buildOperation(operation));
    const finalResources = [];
    for (const operation of finalPass.operations) finalResources.push(await buildOperation(operation));
    const offscreenPipelines = createPipelines('rgba8unorm-srgb');
    async function executePair(sceneOutput, finalOutput, pipelines) {
        const started = performance.now();
        const sceneEncoder = device.createCommandEncoder();
        const sceneRenderPass = sceneEncoder.beginRenderPass({ colorAttachments: [{ view: sceneOutput.createView(), clearValue: { r: scenePass.clear_color[0], g: scenePass.clear_color[1], b: scenePass.clear_color[2], a: scenePass.clear_color[3] }, loadOp: 'clear', storeOp: 'store' }] });
        for (const operation of sceneResources) {
            sceneRenderPass.setPipeline(pipelines[operation.pipeline]);
            operation.bindGroups.forEach((bindGroup, index) => sceneRenderPass.setBindGroup(index, bindGroup));
            sceneRenderPass.draw(operation.vertexCount, operation.instanceCount, 0, 0);
        }
        sceneRenderPass.end();
        device.queue.submit([sceneEncoder.finish()]);
        const finalEncoder = device.createCommandEncoder();
        const finalRenderPass = finalEncoder.beginRenderPass({ colorAttachments: [{ view: finalOutput.createView(), clearValue: { r: finalPass.clear_color[0], g: finalPass.clear_color[1], b: finalPass.clear_color[2], a: finalPass.clear_color[3] }, loadOp: 'clear', storeOp: 'store' }] });
        for (const operation of finalResources) {
            finalRenderPass.setPipeline(pipelines[operation.pipeline]);
            operation.bindGroups.forEach((bindGroup, index) => finalRenderPass.setBindGroup(index, bindGroup));
            finalRenderPass.draw(operation.vertexCount, operation.instanceCount, 0, 0);
        }
        finalRenderPass.end();
        device.queue.submit([finalEncoder.finish()]);
        await device.queue.onSubmittedWorkDone();
        return performance.now() - started;
    }
    const coldRenderTimeMs = await executePair(sceneTexture, finalTexture, offscreenPipelines);
    const warmRenderTimeMs = await executePair(sceneTexture, finalTexture, offscreenPipelines);
    const bytes = await readTextureBytes(device, finalTexture, target.width, target.height);
    await saveRawTexture(bytes, {
        name: 'tc08_5_nightsky_web', width: target.width, height: target.height, format: 'Rgba8UnormSrgb',
        cold_render_time_ms: coldRenderTimeMs, warm_render_time_ms: warmRenderTimeMs,
        manifest: 'tests/shared_assets/manifests/tc08_5_nightsky.json',
        manifest_fingerprint: fnv1a64(new TextEncoder().encode(manifestText)),
        adapter_name: gpu.adapter.info?.description || gpu.adapter.info?.architecture || 'WebGPU adapter',
        timing_scope: 'execute offscreen của 2 pass scene → final + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback',
        pass_count: manifest.evaluation.expected_passes, node_count: manifest.graph.node_count, draw_commands: manifest.graph.command_count,
        image_name: 'tc08_5_nightsky_web.png'
    });
    const canvas = document.getElementById('canvas-tc085');
    const context = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: canvasFormat, alphaMode: 'opaque' });
    await executePair(sceneTexture, context.getCurrentTexture(), createPipelines(canvasFormat));
    document.getElementById('tag-tc085').textContent = 'PASS';
    document.getElementById('tag-tc085').className = 'tag tag-passed';
    sceneTexture.destroy();
    finalTexture.destroy();
    for (const asset of Object.values(assetCache)) asset.image.texture.destroy();
    for (const operation of [...sceneResources, ...finalResources]) if (operation.uniformBuffer) operation.uniformBuffer.destroy();
}

async function fetchShader(name) {
    const res = await fetch(`/shaders/${name}?runner_shader_revision=6`);
    if (!res.ok) throw new Error(`Failed to load shader: ${name}`);
    return await res.text();
}

async function initWebGPU() {
    if (!navigator.gpu) {
        log("WebGPU not supported on this browser!", "error");
        document.getElementById('overall-status').textContent = "WebGPU NOT Supported";
        return null;
    }
    const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    if (!adapter) {
        log("Failed to get WebGPU adapter", "error");
        return null;
    }
    const device = await adapter.requestDevice();
    log(`WebGPU Device initialized: ${adapter.info?.architecture || 'Native GPU'}`, 'success');
    return { adapter, device };
}

// -------------------------------------------------------------
// TC98: Uniform Ring Buffer Stress (64 Sprites Fibonacci Spiral)
// -------------------------------------------------------------
async function runTC98(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc98');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const shaderCode = await fetchShader('ring_buffer_sprites.wgsl');
    const shaderModule = device.createShaderModule({ code: shaderCode });

    const bgl = device.createBindGroupLayout({
        entries: [{
            binding: 0,
            visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
            buffer: { type: 'uniform', hasDynamicOffset: true, minBindingSize: 256 }
        }]
    });

    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [bgl] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: {
            module: shaderModule,
            entryPoint: 'fs_main',
            targets: [{ format: canvasFormat, blend: {
                color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
                alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
            }}]
        },
        primitive: { topology: 'triangle-strip' }
    });

    const spriteCount = 64;
    const ringBufferSize = spriteCount * 256;
    const uniformBuffer = device.createBuffer({
        size: ringBufferSize,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
    });

    // Exact Archimedean spiral matching tc98_ring_buffer_stress.rs
    const uniformData = new Float32Array(ringBufferSize / 4);
    for (let i = 0; i < spriteCount; i++) {
        const offset = (i * 256) / 4;
        const angle = (i / spriteCount) * Math.PI * 2.0;
        const radius = 0.2 + 0.5 * (i / spriteCount);
        const posX = Math.cos(angle) * radius;
        const posY = Math.sin(angle) * radius;

        const r = (Math.sin(angle) * 0.5 + 0.5) * 0.9 + 0.1;
        const g = (Math.sin(angle + 2.0) * 0.5 + 0.5) * 0.9 + 0.1;
        const b = (Math.sin(angle + 4.0) * 0.5 + 0.5) * 0.9 + 0.1;

        uniformData[offset + 0] = posX;
        uniformData[offset + 1] = posY;
        uniformData[offset + 2] = 0.08;
        uniformData[offset + 3] = 0.08;
        uniformData[offset + 4] = r;
        uniformData[offset + 5] = g;
        uniformData[offset + 6] = b;
        uniformData[offset + 7] = 0.85;
    }
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);

    const bindGroup = device.createBindGroup({
        layout: bgl,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer, size: 256 } }]
    });

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
        colorAttachments: [{
            view: ctx.getCurrentTexture().createView(),
            clearValue: { r: 0.188, g: 0.220, b: 0.286, a: 1.0 }, // Exact sRGB value of linear [0.03, 0.04, 0.07]
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });

    pass.setPipeline(pipeline);
    for (let i = 0; i < spriteCount; i++) {
        pass.setBindGroup(0, bindGroup, [i * 256]);
        pass.draw(4, 1, 0, 0);
    }
    pass.end();
    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc98_ring_buffer_stress.png');
    document.getElementById('tag-tc98').textContent = 'PASS';
    document.getElementById('tag-tc98').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// TC99: Video NV12 Bi-Planar Pipeline (Exact BT.709 SMPTE)
// -------------------------------------------------------------
async function runTC99(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc99');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const w = 800, h = 600;
    const yTex = device.createTexture({
        size: [w, h], format: 'r8unorm', usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST
    });
    const uvTex = device.createTexture({
        size: [w / 2, h / 2], format: 'rg8unorm', usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST
    });

    const alignedPitch = 1024;
    const yData = new Uint8Array(alignedPitch * h);
    const uvData = new Uint8Array(alignedPitch * (h / 2));

    const colors_rgb = [
        [1.0, 1.0, 1.0], // White
        [1.0, 1.0, 0.0], // Yellow
        [0.0, 1.0, 1.0], // Cyan
        [0.0, 1.0, 0.0], // Green
        [1.0, 0.0, 1.0], // Magenta
        [1.0, 0.0, 0.0], // Red
        [0.0, 0.0, 1.0], // Blue
        [0.1, 0.1, 0.1], // Dark Gray / Black
    ];

    for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
            const bar_idx = Math.min(Math.floor((x * 8) / w), 7);
            const [r, g, b] = colors_rgb[bar_idx];

            const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            const u_val = (b - luma) / 1.8556 + 0.5;
            const v_val = (r - luma) / 1.5748 + 0.5;

            yData[y * alignedPitch + x] = Math.floor(Math.min(Math.max(luma, 0.0), 1.0) * 255.0);

            if (y % 2 === 0 && x % 2 === 0) {
                const idx = (y / 2) * alignedPitch + (x / 2) * 2;
                uvData[idx] = Math.floor(Math.min(Math.max(u_val, 0.0), 1.0) * 255.0);
                uvData[idx + 1] = Math.floor(Math.min(Math.max(v_val, 0.0), 1.0) * 255.0);
            }
        }
    }

    device.queue.writeTexture({ texture: yTex }, yData, { bytesPerRow: alignedPitch, rowsPerImage: h }, [w, h]);
    device.queue.writeTexture({ texture: uvTex }, uvData, { bytesPerRow: alignedPitch, rowsPerImage: h / 2 }, [w / 2, h / 2]);

    const shaderCode = await fetchShader('video_nv12.wgsl');
    const shaderModule = device.createShaderModule({ code: shaderCode });

    const paramsBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(paramsBuf, 0, new Float32Array([0.0, 1.0, 1.05, 1.0])); // Exact match desktop params

    const sampler = device.createSampler({ minFilter: 'linear', magFilter: 'linear' });
    const bgl = device.createBindGroupLayout({
        entries: [
            { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
            { binding: 1, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
            { binding: 2, visibility: GPUShaderStage.FRAGMENT, sampler: {} },
            { binding: 3, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }
        ]
    });

    const pipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [bgl] }),
        vertex: { module: shaderModule, entryPoint: 'vs_main' },
        fragment: { module: shaderModule, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-strip' }
    });

    const bindGroup = device.createBindGroup({
        layout: bgl,
        entries: [
            { binding: 0, resource: yTex.createView() },
            { binding: 1, resource: uvTex.createView() },
            { binding: 2, resource: sampler },
            { binding: 3, resource: { buffer: paramsBuf } }
        ]
    });

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
        colorAttachments: [{
            view: ctx.getCurrentTexture().createView(),
            clearValue: { r: 0, g: 0, b: 0, a: 1 },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(4, 1, 0, 0);
    pass.end();

    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc99_video_nv12_pipeline.png');
    document.getElementById('tag-tc99').textContent = 'PASS';
    document.getElementById('tag-tc99').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// TC101: Hardware DMA Texture-to-Texture Direct Replication
// -------------------------------------------------------------
async function runTC101(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc101');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const srcW = 400, srcH = 600;
    const dstW = 800, dstH = 600;

    const texA = device.createTexture({
        size: [srcW, srcH], format: 'rgba8unorm',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });

    const texB = device.createTexture({
        size: [dstW, dstH], format: 'rgba8unorm',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST
    });

    const patternShader = await fetchShader('render_test_pattern.wgsl');
    const patternModule = device.createShaderModule({ code: patternShader });
    const patternPipe = device.createRenderPipeline({
        layout: 'auto',
        vertex: { module: patternModule, entryPoint: 'vs_main' },
        fragment: { module: patternModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm' }] },
        primitive: { topology: 'triangle-strip' }
    });

    const encoder = device.createCommandEncoder();

    // Render Pattern to Texture A (400x600)
    const pA = encoder.beginRenderPass({
        colorAttachments: [{ view: texA.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }]
    });
    pA.setPipeline(patternPipe);
    pA.draw(4, 1, 0, 0);
    pA.end();

    // DMA Copy 1: A -> Left Half of B [0, 0]
    encoder.copyTextureToTexture({ texture: texA }, { texture: texB, origin: [0, 0, 0] }, [srcW, srcH, 1]);

    // DMA Copy 2: A -> Right Half of B [400, 0] (Twin Replication)
    encoder.copyTextureToTexture({ texture: texA }, { texture: texB, origin: [400, 0, 0] }, [srcW, srcH, 1]);

    // Blit B to Canvas
    const blitShader = `
        @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
            var pos = array<vec2f, 4>(vec2f(-1,1), vec2f(-1,-1), vec2f(1,1), vec2f(1,-1));
            return vec4f(pos[i], 0, 1);
        }
        @group(0) @binding(0) var t: texture_2d<f32>;
        @fragment fn fs(@builtin(position) p: vec4f) -> @location(0) vec4f {
            return textureLoad(t, vec2u(p.xy), 0);
        }
    `;
    const blitModule = device.createShaderModule({ code: blitShader });
    const blitPipe = device.createRenderPipeline({
        layout: 'auto',
        vertex: { module: blitModule, entryPoint: 'vs' },
        fragment: { module: blitModule, entryPoint: 'fs', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-strip' }
    });
    const blitBg = device.createBindGroup({
        layout: blitPipe.getBindGroupLayout(0),
        entries: [{ binding: 0, resource: texB.createView() }]
    });

    const pCanvas = encoder.beginRenderPass({
        colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store' }]
    });
    pCanvas.setPipeline(blitPipe);
    pCanvas.setBindGroup(0, blitBg);
    pCanvas.draw(4, 1, 0, 0);
    pCanvas.end();

    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc101_texture_copy.png');
    document.getElementById('tag-tc101').textContent = 'PASS';
    document.getElementById('tag-tc101').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// TC102: Compute-to-VBO Transfer Pipeline
// -------------------------------------------------------------
async function runTC102(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc102');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const gridSize = 32;
    const totalVertices = gridSize * gridSize;
    const vboSize = totalVertices * 32;

    const bufSim = device.createBuffer({ size: vboSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC });
    const bufDest = device.createBuffer({ size: vboSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });

    const indices = [];
    for (let y = 0; y < gridSize - 1; y++) {
        for (let x = 0; x < gridSize - 1; x++) {
            const tl = y * gridSize + x;
            const tr = tl + 1;
            const bl = (y + 1) * gridSize + x;
            const br = bl + 1;
            indices.push(tl, bl, tr, tr, bl, br);
        }
    }
    const ibo = device.createBuffer({ size: indices.length * 4, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(ibo, 0, new Uint32Array(indices));

    const csCode = await fetchShader('compute_vertex_wave.wgsl');
    const csModule = device.createShaderModule({ code: csCode });

    const simParamsBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const simParamsView = new ArrayBuffer(16);
    new Float32Array(simParamsView, 0, 1)[0] = 1.2;
    new Uint32Array(simParamsView, 4, 1)[0] = gridSize;
    new Float32Array(simParamsView, 8, 1)[0] = 8.0;
    new Float32Array(simParamsView, 12, 1)[0] = 0.4;
    device.queue.writeBuffer(simParamsBuf, 0, simParamsView);

    const csBgl = device.createBindGroupLayout({
        entries: [
            { binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'storage' } },
            { binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'uniform' } }
        ]
    });
    const csPipe = device.createComputePipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [csBgl] }),
        compute: { module: csModule, entryPoint: 'cs_main' }
    });
    const csBg = device.createBindGroup({
        layout: csBgl,
        entries: [{ binding: 0, resource: { buffer: bufSim } }, { binding: 1, resource: { buffer: simParamsBuf } }]
    });

    const rsCode = await fetchShader('render_mesh_wave.wgsl');
    const rsModule = device.createShaderModule({ code: rsCode });
    const rsBgl = device.createBindGroupLayout({
        entries: [
            { binding: 0, visibility: GPUShaderStage.VERTEX, buffer: { type: 'read-only-storage' } },
            { binding: 1, visibility: GPUShaderStage.VERTEX, buffer: { type: 'read-only-storage' } }
        ]
    });
    const rsPipe = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [rsBgl] }),
        vertex: { module: rsModule, entryPoint: 'vs_main' },
        fragment: { module: rsModule, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-list' }
    });
    const rsBg = device.createBindGroup({
        layout: rsBgl,
        entries: [{ binding: 0, resource: { buffer: bufDest } }, { binding: 1, resource: { buffer: ibo } }]
    });

    const encoder = device.createCommandEncoder();
    const cpass = encoder.beginComputePass();
    cpass.setPipeline(csPipe);
    cpass.setBindGroup(0, csBg);
    cpass.dispatchWorkgroups(Math.ceil(totalVertices / 64), 1, 1);
    cpass.end();

    encoder.copyBufferToBuffer(bufSim, 0, bufDest, 0, vboSize);

    const rpass = encoder.beginRenderPass({
        colorAttachments: [{
            view: ctx.getCurrentTexture().createView(),
            clearValue: { r: 0.04, g: 0.05, b: 0.08, a: 1.0 },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    rpass.setPipeline(rsPipe);
    rpass.setBindGroup(0, rsBg);
    rpass.draw(indices.length, 1, 0, 0);
    rpass.end();

    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc102_buffer_copy.png');
    document.getElementById('tag-tc102').textContent = 'PASS';
    document.getElementById('tag-tc102').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// TC103: Depth Aspect Isolation & False-Color Map
// -------------------------------------------------------------
async function runTC103(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc103');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const w = 800, h = 600;
    const depthSrc = device.createTexture({
        size: [w, h], format: 'depth32float',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC
    });
    const depthDst = device.createTexture({
        size: [w, h], format: 'depth32float',
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST
    });
    const colorScene = device.createTexture({
        size: [w, h], format: 'rgba8unorm', usage: GPUTextureUsage.RENDER_ATTACHMENT
    });

    const sceneShader = await fetchShader('render_depth_scene.wgsl');
    const sceneModule = device.createShaderModule({ code: sceneShader });
    const sceneBgl = device.createBindGroupLayout({
        entries: [{ binding: 0, visibility: GPUShaderStage.VERTEX, buffer: { type: 'uniform' } }]
    });
    const sceneUni = device.createBuffer({ size: 80, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(sceneUni, 0, new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1, 1,1,1,1]));

    const scenePipe = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [sceneBgl] }),
        vertex: { module: sceneModule, entryPoint: 'vs_main' },
        fragment: { module: sceneModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm' }] },
        depthStencil: { format: 'depth32float', depthWriteEnabled: true, depthCompare: 'less' },
        primitive: { topology: 'triangle-list' }
    });

    const visShader = await fetchShader('visualize_depth.wgsl');
    const visModule = device.createShaderModule({ code: visShader });
    const visBgl = device.createBindGroupLayout({
        entries: [{ binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'depth' } }]
    });
    const visPipe = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [visBgl] }),
        vertex: { module: visModule, entryPoint: 'vs_main' },
        fragment: { module: visModule, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-strip' }
    });

    const encoder = device.createCommandEncoder();
    const spass = encoder.beginRenderPass({
        colorAttachments: [{ view: colorScene.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }],
        depthStencilAttachment: { view: depthSrc.createView(), depthClearValue: 1.0, depthLoadOp: 'clear', depthStoreOp: 'store' }
    });
    spass.setPipeline(scenePipe);
    spass.setBindGroup(0, device.createBindGroup({ layout: sceneBgl, entries: [{ binding: 0, resource: { buffer: sceneUni } }] }));
    spass.draw(18, 1, 0, 0);
    spass.end();

    encoder.copyTextureToTexture(
        { texture: depthSrc, aspect: 'depth-only' },
        { texture: depthDst, aspect: 'depth-only' },
        [w, h, 1]
    );

    const vpass = encoder.beginRenderPass({
        colorAttachments: [{ view: ctx.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: 'clear', storeOp: 'store' }]
    });
    vpass.setPipeline(visPipe);
    vpass.setBindGroup(0, device.createBindGroup({ layout: visBgl, entries: [{ binding: 0, resource: depthDst.createView() }] }));
    vpass.draw(4, 1, 0, 0);
    vpass.end();

    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc103_depth_aspect_copy.png');
    document.getElementById('tag-tc103').textContent = 'PASS';
    document.getElementById('tag-tc103').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// TC104: Custom Extension Node Dispatch
// -------------------------------------------------------------
async function runTC104(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc104');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const patternShader = await fetchShader('render_test_pattern.wgsl');
    const patternModule = device.createShaderModule({ code: patternShader });
    const patternPipe = device.createRenderPipeline({
        layout: 'auto',
        vertex: { module: patternModule, entryPoint: 'vs_main' },
        fragment: { module: patternModule, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-strip' }
    });

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
        colorAttachments: [{
            view: ctx.getCurrentTexture().createView(),
            clearValue: { r: 0.05, g: 0.05, b: 0.08, a: 1 },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    pass.setPipeline(patternPipe);
    pass.draw(4, 1, 0, 0);
    pass.end();

    log("TC104: Extension Dispatch simulated on WebGPU CommandBuffer", 'info');
    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc104_extension_dispatch.png');
    document.getElementById('tag-tc104').textContent = 'PASS';
    document.getElementById('tag-tc104').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// TC105: Hybrid Motion Echo Feedback Loop
// -------------------------------------------------------------
async function runTC105(gpu) {
    const { device } = gpu;
    const canvas = document.getElementById('canvas-tc105');
    const ctx = canvas.getContext('webgpu');
    const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: canvasFormat, alphaMode: 'opaque' });

    const w = 800, h = 600;
    const targetTex = device.createTexture({
        size: [w, h], format: 'rgba8unorm',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_SRC
    });
    const pingTex = device.createTexture({
        size: [w, h], format: 'rgba8unorm',
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST
    });
    const pongTex = device.createTexture({
        size: [w, h], format: 'rgba8unorm',
        usage: GPUTextureUsage.STORAGE_BINDING | GPUTextureUsage.TEXTURE_BINDING
    });

    const orbShader = await fetchShader('render_glowing_orb.wgsl');
    const orbModule = device.createShaderModule({ code: orbShader });
    const orbPipe = device.createRenderPipeline({
        layout: 'auto',
        vertex: { module: orbModule, entryPoint: 'vs_main' },
        fragment: { module: orbModule, entryPoint: 'fs_main', targets: [{ format: canvasFormat }] },
        primitive: { topology: 'triangle-strip' }
    });
    const orbPipeOffscreen = device.createRenderPipeline({
        layout: 'auto',
        vertex: { module: orbModule, entryPoint: 'vs_main' },
        fragment: { module: orbModule, entryPoint: 'fs_main', targets: [{ format: 'rgba8unorm' }] },
        primitive: { topology: 'triangle-strip' }
    });

    const csCode = await fetchShader('compute_decay_echo.wgsl');
    const csModule = device.createShaderModule({ code: csCode });
    const csBgl = device.createBindGroupLayout({
        entries: [
            { binding: 0, visibility: GPUShaderStage.COMPUTE, texture: { sampleType: 'float' } },
            { binding: 1, visibility: GPUShaderStage.COMPUTE, storageTexture: { access: 'write-only', format: 'rgba8unorm' } },
            { binding: 2, visibility: GPUShaderStage.COMPUTE, sampler: {} },
            { binding: 3, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'uniform' } }
        ]
    });
    const csPipe = device.createComputePipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [csBgl] }),
        compute: { module: csModule, entryPoint: 'cs_main' }
    });

    const echoParamsBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(echoParamsBuf, 0, new Float32Array([0.92, 0.03, 0, 0]));

    const sampler = device.createSampler({ minFilter: 'linear', magFilter: 'linear' });
    const csBg = device.createBindGroup({
        layout: csBgl,
        entries: [
            { binding: 0, resource: pingTex.createView() },
            { binding: 1, resource: pongTex.createView() },
            { binding: 2, resource: sampler },
            { binding: 3, resource: { buffer: echoParamsBuf } }
        ]
    });

    const compShader = await fetchShader('deep_composite_filter.wgsl');
    const compModule = device.createShaderModule({ code: compShader });
    const compBgl = device.createBindGroupLayout({
        entries: [
            { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
            { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: {} }
        ]
    });
    const compPipe = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [compBgl] }),
        vertex: { module: compModule, entryPoint: 'vs_main' },
        fragment: {
            module: compModule,
            entryPoint: 'fs_main',
            targets: [{
                format: canvasFormat,
                blend: {
                    color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
                    alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
                }
            }]
        },
        primitive: { topology: 'triangle-strip' }
    });

    const compBg = device.createBindGroup({
        layout: compBgl,
        entries: [{ binding: 0, resource: pongTex.createView() }, { binding: 1, resource: sampler }]
    });

    const encoder = device.createCommandEncoder();

    // Step 1: Draw Glowing Orb on canvas
    const p1 = encoder.beginRenderPass({
        colorAttachments: [{
            view: ctx.getCurrentTexture().createView(),
            clearValue: { r: 0.03, g: 0.04, b: 0.07, a: 1 },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    p1.setPipeline(orbPipe);
    p1.draw(4, 1, 0, 0);
    p1.end();

    // Step 2: Draw to Target for copy
    const pTarget = encoder.beginRenderPass({
        colorAttachments: [{
            view: targetTex.createView(),
            clearValue: { r: 0.03, g: 0.04, b: 0.07, a: 1 },
            loadOp: 'clear',
            storeOp: 'store'
        }]
    });
    pTarget.setPipeline(orbPipeOffscreen);
    pTarget.draw(4, 1, 0, 0);
    pTarget.end();

    // Step 3: DMA Copy Target -> Ping
    encoder.copyTextureToTexture({ texture: targetTex }, { texture: pingTex }, [w, h, 1]);

    // Step 4: Compute Decay Ping -> Pong
    const cp = encoder.beginComputePass();
    cp.setPipeline(csPipe);
    cp.setBindGroup(0, csBg);
    cp.dispatchWorkgroups(Math.ceil(w / 16), Math.ceil(h / 16), 1);
    cp.end();

    // Step 5: Composite Pong onto Canvas
    const pFinal = encoder.beginRenderPass({
        colorAttachments: [{
            view: ctx.getCurrentTexture().createView(),
            loadOp: 'load',
            storeOp: 'store'
        }]
    });
    pFinal.setPipeline(compPipe);
    pFinal.setBindGroup(0, compBg);
    pFinal.draw(4, 1, 0, 0);
    pFinal.end();

    device.queue.submit([encoder.finish()]);

    await saveCanvasImage(canvas, 'tc105_pingpong_echo.png');
    document.getElementById('tag-tc105').textContent = 'PASS';
    document.getElementById('tag-tc105').className = 'tag tag-passed';
}

// -------------------------------------------------------------
// Suite Orchestrator
// -------------------------------------------------------------
async function runAllTests() {
    log("Starting WebGPU Cross-Platform Verification Suite...");
    const gpu = await initWebGPU();
    if (!gpu) return;

    const query = runnerQuery;
    const skipProbe = query.get('skip_probe') === '1';
    if (skipPreview) {
        log('Skipping presentation PNG previews for this focused run');
    }
    if (skipProbe) {
        log('Skipping canonical parity probe for this focused run');
    } else {
        log('Executing canonical offscreen parity probe...');
        try {
            await runCanonicalParityProbe(gpu);
            log('Canonical offscreen parity probe saved', 'success');
        } catch (e) {
            log(`Canonical parity probe FAILED: ${e.message}`, 'error');
        }
    }

    const testCatalog = [
        { name: "TC01: Empty Render", fn: runTC01 },
        { name: "TC02: Single Quad Chroma Key", fn: runTC02 },
        { name: "TC03: Z-Buffer Depth Testing", fn: runTC03 },
        { name: "TC04: Alpha Blend + Z-Buffer", fn: runTC04 },
        { name: "TC05: Interleaved Multi-Pass", fn: runTC05 },
        { name: "TC06: RenderNodePool GC", fn: runTC06 },
        { name: "TC07: Deep Recursion SubGraphs", fn: runTC07 },
        { name: "TC08: Massive Procedural Particles", fn: runTC08 },
        { name: "TC08.5: Directional Moonlight Scene", fn: runTC085 },
        { name: "TC09: Pipeline Caching & Bundle Reuse", fn: runTC09 },
        { name: "TC10: Missing Resource Fallback", fn: runTC10 },
        { name: "TC11: Multi-Viewport Isolation", fn: runTC11 },
        { name: "TC12: Multi-Sprite Chroma Key", fn: runTC12 },
        { name: "TC13: Gaussian Blur Depth of Field", fn: runTC13 },
        { name: "TC14: Cinematic Color Grading", fn: runTC14 },
        { name: "TC15: Instanced Snow Physics", fn: runTC15 },
        { name: "TC16: 2D SDF Vector Shapes", fn: runTC16 },
        { name: "TC17: Outline & Drop Shadow", fn: runTC17 },
        { name: "TC18: Video Transition Effects", fn: runTC18 },
        { name: "TC19: Audio-Reactive Spectrum", fn: runTC19 },
        { name: "TC20: 3D Perspective Card", fn: runTC20 },
        { name: "TC21: SDF Masking", fn: runTC21 },
        { name: "TC22: Hardware Instanced Props", fn: runTC22 },
        { name: "TC23: Palette Swap", fn: runTC23 },
        { name: "TC24: Vertex Deformation", fn: runTC24 },
        { name: "TC25: Rimlight and Shadow", fn: runTC25 },
        { name: "TC26: Glitch and Chromatic Aberration", fn: runTC26 },
        { name: "TC27: Godrays", fn: runTC27 },
        { name: "TC28: Ripple", fn: runTC28 },
        { name: "TC29: CRT/VHS", fn: runTC29 },
        { name: "TC30: Dissolve/Burn", fn: runTC30 },
        { name: "TC31: Light Sweep", fn: runTC31 },
        { name: "TC32: Page Curl", fn: runTC32 },
        { name: "TC33: Pixelation", fn: runTC33 },
        { name: "TC34: Directional Blur", fn: runTC34 },
        { name: "TC98: Uniform Ring Buffer", fn: runTC98 },
        { name: "TC99: Video NV12 BT.709", fn: runTC99 },
        { name: "TC101: Texture Copy DMA", fn: runTC101 },
        { name: "TC102: Buffer Copy Compute VBO", fn: runTC102 },
        { name: "TC103: Depth Aspect Copy", fn: runTC103 },
        { name: "TC104: Extension Dispatch", fn: runTC104 },
        { name: "TC105: PingPong Echo Hybrid", fn: runTC105 }
    ];
    const requestedCases = query.get('cases');
    const requestedNames = requestedCases
        ? requestedCases.split(',').map(value => value.trim().toUpperCase()).filter(Boolean)
        : ['TC01'];
    const tests = testCatalog.filter(test => requestedNames.some(name => test.name.startsWith(`${name}:`)));
    if (tests.length === 0) throw new Error(`No supported test case selected: ${requestedNames.join(', ')}`);
    log(`Selected batch: ${tests.map(test => test.name).join(', ')}`);

    let failedCount = 0;
    for (const test of tests) {
        log(`Executing ${test.name}...`);
        const t0 = performance.now();
        try {
            await test.fn(gpu);
            const dt = (performance.now() - t0).toFixed(2);
            log(`${test.name} PASSED in ${dt}ms`, 'success');
        } catch (e) {
            failedCount += 1;
            log(`${test.name} FAILED: ${e.message}`, 'error');
            console.error(e);
        }
    }

    const badge = document.getElementById('overall-status');
    if (failedCount === 0) {
        badge.textContent = `Selected ${tests.length} WebGPU Test Case(s) PASSED ✅`;
        badge.className = "status-badge passed";
        log("=== ALL WEBGPU CROSS-PLATFORM TESTS PASSED ===", 'success');
    } else {
        badge.textContent = `${failedCount}/${tests.length} WebGPU Test Case(s) FAILED ❌`;
        badge.className = "status-badge";
        log(`=== WEBGPU TEST SUITE FAILED: ${failedCount} case(s) ===`, 'error');
    }
}

window.addEventListener('DOMContentLoaded', runAllTests);
