pub fn read(mut view: &mut [u8], mut offset: usize, mmap: &memmap2::Mmap) {
	while !view.is_empty() {
		let avaible = std::cmp::min(view.len(), 1020 - offset % 1024);
		view[0..avaible].copy_from_slice(&mmap[offset..(offset + avaible)]);
		view = &mut view[avaible..];
		offset += avaible + 4;
	}
}
