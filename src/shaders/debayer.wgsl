
@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<vec4<f32>>;
// var<storage, read_write> output: array<f32>;


// struct PushConstants{
// 	width: i32,
// 	height: i32,
// }

// var<push_constant> pc: PushConstants;

var<workgroup> local: array<f32, #expr{(WG_X + 2 * PADDING) * (WG_Y + 2 * PADDING)}>;

fn reflect(idx: i32, max: i32) -> i32{
	if idx < 0{
		return -(idx + 1);
	} else if idx >= max{
		return 2 * max - 1 - idx;
	} else {
		return idx;
	}
}

fn reflect_vec(idx: vec2<i32>, max: vec2<i32>) -> vec2<i32>{
	return vec2(reflect(idx.x, max.x), reflect(idx.y, max.y));
}

const local_height = #expr{WG_X + 2 * PADDING};
const local_width = #expr{WG_Y + 2 * PADDING};
const local_size = #expr{(WG_X + 2 * PADDING) * (WG_Y + 2 * PADDING)};

fn local_flat(coord_x: i32, coord_y: i32) -> i32{
	return coord_x * local_width + coord_y;
}

fn malvar_r(center: vec2<i32>) -> vec3<f32>{
	let r = local[local_flat(center.x, center.y)];
	let g = (
		4.0 * local[local_flat(center.x, center.y)] -
		local[local_flat(center.x - 2, center.y)] -
		local[local_flat(center.x, center.y - 2)] -
		local[local_flat(center.x + 2, center.y)] -
		local[local_flat(center.x, center.y + 2)] +
		2. * (
			local[local_flat(center.x + 1, center.y)] +
			local[local_flat(center.x, center.y + 1)] +
			local[local_flat(center.x - 1, center.y)] +
			local[local_flat(center.x, center.y - 1)]
		)
	) / 8.0;
	
    let b = (
		6.0 * local[local_flat(center.x, center.y)] -
		3.0 * (
			local[local_flat(center.x - 2, center.y)] +
			local[local_flat(center.x, center.y - 2)] +
			local[local_flat(center.x + 2, center.y)] +
			local[local_flat(center.x, center.y + 2)]
		) / 2.0 +
		2.0 * (
			local[local_flat(center.x - 1, center.y - 1)] +
			local[local_flat(center.x - 1, center.y + 1)] +
			local[local_flat(center.x + 1, center.y - 1)] +
			local[local_flat(center.x + 1, center.y + 1)]
		)
	) / 8.;

	return vec3(r, g, b);
}

fn malvar_gr(center: vec2<i32>) -> vec3<f32>{
	let r = (
		5.0 * local[local_flat(center.x, center.y)] -
		local[local_flat(center.x, center.y - 2)] -
		local[local_flat(center.x - 1, center.y - 1)] -
		local[local_flat(center.x + 1, center.y - 1)] -
		local[local_flat(center.x - 1, center.y + 1)] -
		local[local_flat(center.x + 1, center.y + 1)] -
		local[local_flat(center.x, center.y + 2)] +
		(
			local[local_flat(center.x - 2, center.y)] +
			local[local_flat(center.x + 2, center.y)]
		) / 2.0 +
		4.0 * (
			local[local_flat(center.x, center.y - 1)] +
			local[local_flat(center.x, center.y + 1)]
		)
	) / 8.0;
	
	let g = local[local_flat(center.x, center.y)];
	
    let b = (
		5.0 * local[local_flat(center.x, center.y)] -
		local[local_flat(center.x - 2, center.y)] -
		local[local_flat(center.x - 1, center.y - 1)] -
		local[local_flat(center.x - 1, center.y + 1)] -
		local[local_flat(center.x + 2, center.y)] -
		local[local_flat(center.x + 1, center.y - 1)] -
		local[local_flat(center.x + 1, center.y + 1)] +
		(
			local[local_flat(center.x, center.y - 2)] +
			local[local_flat(center.x, center.y + 2)]
		) / 2.0 +
		4.0 * (
			local[local_flat(center.x - 1, center.y)] +
			local[local_flat(center.x + 1, center.y)]
		)
	) / 8.0;

	return vec3(r, g, b);
}

fn malvar_gb(center: vec2<i32>) -> vec3<f32>{
    let r = (
		5.0 * local[local_flat(center.x, center.y)] -
		local[local_flat(center.x - 2, center.y)] -
		local[local_flat(center.x - 1, center.y - 1)] -
		local[local_flat(center.x - 1, center.y + 1)] -
		local[local_flat(center.x + 2, center.y)] -
		local[local_flat(center.x + 1, center.y - 1)] -
		local[local_flat(center.x + 1, center.y + 1)] +
		(
			local[local_flat(center.x, center.y - 2)] +
			local[local_flat(center.x, center.y + 2)]
		) / 2.0 +
		4.0 * (
			local[local_flat(center.x - 1, center.y)] +
			local[local_flat(center.x + 1, center.y)]
		)
	) / 8.0;

	let g = local[local_flat(center.x, center.y)];

	let b = (
		5.0 * local[local_flat(center.x, center.y)] -
		local[local_flat(center.x, center.y - 2)] -
		local[local_flat(center.x - 1, center.y - 1)] -
		local[local_flat(center.x + 1, center.y - 1)] -
		local[local_flat(center.x - 1, center.y + 1)] -
		local[local_flat(center.x + 1, center.y + 1)] -
		local[local_flat(center.x, center.y + 2)] + (
			local[local_flat(center.x - 2, center.y)] +
			local[local_flat(center.x + 2, center.y)]
		) / 2.0 + 4.0 * (
			local[local_flat(center.x, center.y - 1)] +
			local[local_flat(center.x, center.y + 1)]
		)) / 8.0;

	return vec3(r, g, b);
}

fn malvar_b(center: vec2<i32>) -> vec3<f32>{
	let r = (
		6.0 * local[local_flat(center.x, center.y)] -
		3.0 * (
			local[local_flat(center.x - 2, center.y)] +
			local[local_flat(center.x, center.y - 2)] +
			local[local_flat(center.x + 2, center.y)] +
			local[local_flat(center.x, center.y + 2)]
		) / 2.0 +
		2.0 * (
			local[local_flat(center.x - 1, center.y - 1)] +
			local[local_flat(center.x - 1, center.y + 1)] +
			local[local_flat(center.x + 1, center.y - 1)] +
			local[local_flat(center.x + 1, center.y + 1)]
		)
	) / 8.0;

	let g = (
		4.0 * local[local_flat(center.x, center.y)] -
		local[local_flat(center.x - 2, center.y)] -
		local[local_flat(center.x, center.y - 2)] -
		local[local_flat(center.x + 2, center.y)] -
		local[local_flat(center.x, center.y + 2)] +
		2.0 * (
			local[local_flat(center.x + 1, center.y)] +
			local[local_flat(center.x, center.y + 1)] +
			local[local_flat(center.x - 1, center.y)] +
			local[local_flat(center.x, center.y - 1)]
		)
	) / 8.0;

	let b = local[local_flat(center.x, center.y)];
	return vec3(r, g, b);
}



@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(
	@builtin(global_invocation_id) global_id: vec3<u32>,
	@builtin(local_invocation_id) local_id: vec3<u32>,
	@builtin(local_invocation_index) local_index: u32,
	@builtin(workgroup_id) wg_id: vec3<u32>,
){
	let global_bounds = vec2(#HEIGHT, #WIDTH);
	if any(global_id < vec3(0u)) || any(global_id >= vec3(vec2<u32>(global_bounds.xy), 1u)){
		return;
	}

	let offset_to_global = vec2<i32>(wg_id.xy) * vec2(i32(#WG_X), i32(#WG_Y)) - vec2(#PADDING);

	for (var i = 0u; i < 2u; i += 1u){
		let local_flat = i32(local_index + #WG_X * #WG_Y * i);
		if local_flat >= local_size{
			break;
		}
		
		let local_coord = vec2(
			i32((local_flat) / local_width),
			i32((local_flat) % local_width),
		);
		let global_coord = reflect_vec(local_coord + offset_to_global, global_bounds);

		let global_flat = global_coord.x * #WIDTH + global_coord.y;
		local[local_flat] = input[global_flat];
	}

	workgroupBarrier();

	let local_center = vec2<i32>(local_id.xy) + vec2(#PADDING);
	
	var color: vec3<f32>;

	let mod_row = global_id.x % 2u;
	let mod_col = global_id.y % 2u;

	if mod_row == 0u && mod_col == 0u{
		color = malvar_r(local_center);
	} else if mod_row == 0u && mod_col == 1u {
		color = malvar_gr(local_center);
	} else if mod_row == 1u && mod_col == 0u {
		color = malvar_gb(local_center);
	} else {
		color = malvar_b(local_center);
	}

	let global_flat = (i32(global_id.x) * #WIDTH + i32(global_id.y));

	output[global_flat] = vec4(color, 1.);
	// output[global_flat * 3] = color.r;
	// output[global_flat * 3 + 1] = color.g;
	// output[global_flat * 3 + 2] = color.b;
}

