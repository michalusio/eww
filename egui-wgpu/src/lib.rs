//! [egui](https://github.com/emilk/egui) rendering support
//! for [wgpu](https://github.com/gfx-rs/wgpu-rs).

use wgpu::util::DeviceExt as _;
use wgpu_util::{BufferPool, BufferPoolDescriptor};

#[macro_use]
extern crate bytemuck;

use std::num::NonZeroU32;

/// egui renderer
pub struct Renderer {
    render_pipeline: wgpu::RenderPipeline,

    vbo_pool: BufferPool,
    ibo_pool: BufferPool,

    globals_ubo: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: Option<wgpu::BindGroup>,

    user_textures: Vec<Option<wgpu::BindGroup>>,
    texture_version: Option<u64>,
    next_user_texture_id: u64,
    pending_user_textures: Vec<(u64, egui::Texture)>,
}

impl Renderer {
    /// Creates a new egui renderer
    ///
    /// `output_format` needs to be either [`wgpu::TextureFormat::Rgba8UnormSrgb`] or
    /// [`wgpu::TextureFormat::Bgra8UnormSrgb`]. Panics otherwise.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        if !(output_format == wgpu::TextureFormat::Rgba8UnormSrgb
            || output_format == wgpu::TextureFormat::Bgra8UnormSrgb)
        {
            panic!("`output_format` needs to be either `wgpu::TextureFormat::{{Rgba8UnormSrgb, Brga8UnormSrgb}}`, but currently is `{:?}`", output_format);
        }

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("egui_wgpu_uniform_buffer"),
            contents: bytemuck::bytes_of(&Globals {
                screen_size: [0.0, 0.0],
            }),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("egui_wgpu_texture_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui_wgpu_uniform_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Uniform,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    },
                ],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_wgpu_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui_wgpu_texture_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let vs_module = device.create_shader_module(&wgpu::include_spirv!("shader/egui.vert.spv"));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!("shader/egui.frag.spv"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("egui_wgpu_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("egui_wgpu_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                entry_point: "main",
                module: &vs_module,
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 5 * 4,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        // vec2 position
                        0 => Float32x2,
                        // vec2 texture coordinates
                        1 => Float32x2,
                        // uint color
                        2 => Uint32,
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::default(),
                cull_mode: None,
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::default(),
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                count: 1,
                mask: !0,
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        let vbo_pool = BufferPool::new(&BufferPoolDescriptor {
            label: Some("egui_wgpu_vertex_buffer_pool"),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let ibo_pool = BufferPool::new(&BufferPoolDescriptor {
            label: Some("egui_wgpu_index_buffer_pool"),
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
        });

        Self {
            render_pipeline,
            vbo_pool,
            ibo_pool,
            globals_ubo: uniform_buffer,
            globals_bind_group: uniform_bind_group,
            texture_bind_group_layout,
            texture_version: None,
            texture_bind_group: None,
            next_user_texture_id: 0,
            pending_user_textures: Vec::new(),
            user_textures: Vec::new(),
        }
    }

    /// Renders all egui meshes onto render_target.
    pub fn render<'a, M, T>(&mut self, desc: RenderDescriptor<'a, M, T>)
    where
        M: Iterator<Item = &'a egui::ClippedMesh> + Clone,
        T: Iterator<Item = &'a egui::Texture>,
    {
        let RenderDescriptor {
            device,
            queue,
            encoder,
            render_target,
            meshes,
            screen_descriptor,
            load_operation,
            textures_to_update,
        } = desc;

        for texture in textures_to_update {
            self.update_texture(device, queue, texture);
        }
        self.update_user_textures(device, queue);

        Globals::update_ubo(&self.globals_ubo, queue, screen_descriptor);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_operation,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            label: Some("egui_wgpu_render_pass"),
        });

        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);

        let scale_factor = screen_descriptor.scale_factor;
        let physical_width = screen_descriptor.physical_width;
        let physical_height = screen_descriptor.physical_height;

        self.vbo_pool.clear();
        self.ibo_pool.clear();
        for egui::ClippedMesh(_, mesh) in meshes.clone() {
            self.vbo_pool
                .upload(device, queue, util::mesh_vertex_data(mesh));
            self.ibo_pool
                .upload(device, queue, util::mesh_index_data(mesh));
        }

        for (i, egui::ClippedMesh(clip_rect, mesh)) in meshes.enumerate() {
            pass.set_bind_group(1, self.get_texture_bind_group(mesh.texture_id), &[]);

            // Transform clip rect to physical pixels.
            let clip_min_x = scale_factor * clip_rect.min.x;
            let clip_min_y = scale_factor * clip_rect.min.y;
            let clip_max_x = scale_factor * clip_rect.max.x;
            let clip_max_y = scale_factor * clip_rect.max.y;

            // Make sure clip rect can fit within an `u32`.
            let clip_min_x = clip_min_x.clamp(0.0, physical_width as f32);
            let clip_min_y = clip_min_y.clamp(0.0, physical_height as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, physical_width as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, physical_height as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            let width = (clip_max_x - clip_min_x).max(1);
            let height = (clip_max_y - clip_min_y).max(1);

            {
                // clip scissor rectangle to target size
                let x = clip_min_x.min(physical_width);
                let y = clip_min_y.min(physical_height);
                let width = width.min(physical_width - x);
                let height = height.min(physical_height - y);

                // skip rendering with zero-sized clip areas
                if width == 0 || height == 0 {
                    continue;
                }

                pass.set_scissor_rect(x, y, width, height);
            }

            let vbo = self.vbo_pool.get(i).unwrap();
            let ibo = self.ibo_pool.get(i).unwrap();

            pass.set_vertex_buffer(0, vbo.slice(..));
            pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
    }

    /// Registers a `wgpu::Texture` in the renderer.
    pub fn register_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
    ) -> egui::TextureId {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(
                format!("egui_wgpu_texture_bind_group_{}", self.next_user_texture_id).as_str(),
            ),
            layout: &self.texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });
        let texture_id = egui::TextureId::User(self.next_user_texture_id);
        self.user_textures.push(Some(bind_group));
        self.next_user_texture_id += 1;

        texture_id
    }

    /// Unregisters the texture.
    pub fn unregister_texture(&mut self, id: egui::TextureId) {
        if let egui::TextureId::User(id) = id {
            self.user_textures
                .get_mut(id as usize)
                .and_then(|option| option.take());
        }
    }
}

impl Renderer {
    // Updates the texture used by egui for the fonts etc.
    fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_texture: &egui::Texture,
    ) {
        // Don't update the texture if it hasn't changed.
        if self.texture_version == Some(egui_texture.version) {
            return;
        }
        // we need to convert the texture into rgba_srgb format
        let mut pixels: Vec<u8> = Vec::with_capacity(egui_texture.pixels.len() * 4);
        for srgba in egui_texture.srgba_pixels() {
            pixels.push(srgba.r());
            pixels.push(srgba.g());
            pixels.push(srgba.b());
            pixels.push(srgba.a());
        }
        let egui_texture = egui::Texture {
            version: egui_texture.version,
            width: egui_texture.width,
            height: egui_texture.height,
            pixels,
        };
        let bind_group = self.create_egui_texture(device, queue, &egui_texture, "egui_font");

        self.texture_version = Some(egui_texture.version);
        self.texture_bind_group = Some(bind_group);
    }

    // Updates the user textures that the app allocated.
    fn update_user_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let pending_user_textures = std::mem::take(&mut self.pending_user_textures);
        for (id, texture) in pending_user_textures {
            let bind_group = self.create_egui_texture(
                device,
                queue,
                &texture,
                format!("user_texture{}", id).as_str(),
            );
            self.user_textures.push(Some(bind_group));
        }
    }

    // TODO: needed?
    pub fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[egui::Color32],
    ) -> egui::TextureId {
        let id = self.next_user_texture_id;
        self.next_user_texture_id += 1;

        let mut pixels = vec![0u8; srgba_pixels.len() * 4];
        for (target, given) in pixels.chunks_exact_mut(4).zip(srgba_pixels.iter()) {
            target.copy_from_slice(&given.to_array());
        }

        let (width, height) = size;
        self.pending_user_textures.push((
            id,
            egui::Texture {
                version: 0,
                width,
                height,
                pixels,
            },
        ));

        egui::TextureId::User(id)
    }

    fn get_texture_bind_group(&self, texture_id: egui::TextureId) -> &wgpu::BindGroup {
        match texture_id {
            egui::TextureId::Egui => self
                .texture_bind_group
                .as_ref()
                .expect("egui texture was not set before the first draw"),
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                &(self
                    .user_textures
                    .get(id)
                    .unwrap_or_else(|| panic!("user texture {} not found", id))
                    .as_ref()
                    .unwrap_or_else(|| panic!("user texture {} freed", id)))
            }
        }
    }

    // Assumes egui_texture contains srgb data.
    // This does not match how egui::Texture is documented as of writing, but this is how it is used for user textures.
    fn create_egui_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_texture: &egui::Texture,
        label: &str,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: egui_texture.width as u32,
            height: egui_texture.height as u32,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(format!("{}_texture", label).as_str()),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            egui_texture.pixels.as_slice(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(
                    (egui_texture.pixels.len() / egui_texture.height) as u32,
                ),
                rows_per_image: NonZeroU32::new(egui_texture.height as u32),
            },
            size,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(format!("{}_texture_bind_group", label).as_str()),
            layout: &self.texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });

        bind_group
    }
}

#[derive(Debug)]
pub struct RenderDescriptor<'a, MeshIterator, TextureIterator>
where
    MeshIterator: Iterator<Item = &'a egui::ClippedMesh> + Clone,
    TextureIterator: Iterator<Item = &'a egui::Texture>,
{
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    encoder: &'a mut wgpu::CommandEncoder,
    render_target: &'a wgpu::TextureView,
    screen_descriptor: ScreenDescriptor,
    load_operation: wgpu::LoadOp<wgpu::Color>,
    meshes: MeshIterator,
    textures_to_update: TextureIterator,
}

/// Information about the screen used for rendering.
#[derive(Clone, Copy, Debug)]
pub struct ScreenDescriptor {
    /// Width of the window in physical pixel.
    pub physical_width: u32,
    /// Height of the window in physical pixel.
    pub physical_height: u32,
    /// HiDPI scale factor.
    pub scale_factor: f32,
}

impl ScreenDescriptor {
    fn logical_size_f32(self) -> [f32; 2] {
        let width = self.physical_width as f32 / self.scale_factor;
        let height = self.physical_height as f32 / self.scale_factor;
        [width, height]
    }
}

/// Globals used when rendering.
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
struct Globals {
    screen_size: [f32; 2],
}

impl Globals {
    fn update_ubo(ubo: &wgpu::Buffer, queue: &wgpu::Queue, screen_descriptor: ScreenDescriptor) {
        queue.write_buffer(
            ubo,
            0,
            bytemuck::bytes_of(&Globals {
                screen_size: screen_descriptor.logical_size_f32(),
            }),
        );
    }
}

/// A egui texture render target
pub struct RenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,

    texture_id: egui::TextureId,
}

impl RenderTarget {
    // Is `texture_format ==  TextureFormat::Rgba8UnormSrgb` a requirement?
    pub fn new(
        device: &wgpu::Device,
        renderer: &mut Renderer,
        texture_format: wgpu::TextureFormat,
        size: [u32; 2],
    ) -> Self {
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };

        let descriptor = wgpu::TextureDescriptor {
            label: Some("egui_wgpu_render_target_texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        };

        let texture = device.create_texture(&descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let gui_wgpu_texture_layout_descriptor = wgpu::BindGroupLayoutDescriptor {
            label: Some("egui_wgpu_render_target_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
        };

        let _bind_group_layout =
            device.create_bind_group_layout(&gui_wgpu_texture_layout_descriptor);

        let texture_id = renderer.register_texture(device, &texture);

        Self {
            texture,
            view,

            texture_id,
        }
    }

    pub fn texture_id(&self) -> egui::TextureId {
        self.texture_id
    }
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

mod util {
    // This will be useable as soon as another release of bytemuck is available.

    //use bytemuck::TransparentWrapper;
    //
    //#[derive(Copy, Clone, TransparentWrapper)]
    //#[repr(transparent)]
    //struct VertexWrapper(egui::epaint::Vertex);
    //
    //// NOTE: Not yet derivable in combination with `bytemuck::TransparentWrapper`
    //unsafe impl bytemuck::Zeroable for VertexWrapper {}
    //unsafe impl bytemuck::Pod for VertexWrapper {}
    //

    // for the moment we use this as a replacement for `bytemuck::TransparentWrapper::wrap_slice
    fn as_byte_slice<T>(slice: &[T]) -> &[u8] {
        let len = slice.len() * std::mem::size_of::<T>();
        let ptr = slice.as_ptr() as *const u8;
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    pub fn mesh_vertex_data(mesh: &egui::epaint::Mesh) -> &[u8] {
        //bytemuck::cast_slice(VertexWrapper::wrap_slice(&mesh.vertices))
        as_byte_slice(&mesh.vertices)
    }

    pub fn mesh_index_data(mesh: &egui::epaint::Mesh) -> &[u8] {
        bytemuck::cast_slice(&mesh.indices)
    }
}
