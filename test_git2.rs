use git2::Patch;

fn main() {
    let old = b"a\nb\nc\n";
    let new = b"a\nc\nd\n";
    let patch = Patch::from_buffers(old, None, new, None, None).unwrap();
    let num_hunks = patch.num_hunks();
    for i in 0..num_hunks {
        let (hunk, _) = patch.hunk(i).unwrap();
        println!("{} {}", hunk.old_start(), hunk.new_start());
    }
}