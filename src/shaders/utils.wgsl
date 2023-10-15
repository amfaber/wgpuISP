
#export reflect_vec{
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
}

#export is_outside_image{
	fn is_outside_image(global_id: vec3<u32>, global_bounds: vec2<i32>) -> bool{
		return any(global_id < vec3(0u)) || any(global_id >= vec3(vec2<u32>(global_bounds.xy), 1u));
	}
}


#export setup_local{
	fn setup_local(wg_id: vec3<u32>, local_index: u32, global_bounds: vec2<i32>){
		let offset_to_global = vec2<i32>(wg_id.xy) * vec2(i32(#WG_X), i32(#WG_Y)) - vec2(#PADDING);

		var local_flat = i32(local_index);

		while local_flat < local_size{
			let local_coord = vec2(
				i32((local_flat) / local_width),
				i32((local_flat) % local_width),
			);
			let global_coord = reflect_vec(local_coord + offset_to_global, global_bounds);

			let global_flat = global_coord.x * #WIDTH + global_coord.y;
			local[local_flat] = input[global_flat];

			local_flat += i32(#WG_X * #WG_Y);
		}
	
		workgroupBarrier();
	}
}

#export access_local{
	fn access_local(coord_x: i32, coord_y: i32) -> f32{
		return local[coord_x * local_width + coord_y];
	}
}

#export all_utils{
	#import reflect_vec
	#import is_outside_image
	#import setup_local
	#import access_local
}

