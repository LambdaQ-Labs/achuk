//! Achuk `sys` platform host — a self-contained host (std/builtin only) that
//! provides real I/O effects: File.read!, Env.get!, Stdout.line!.
//! Builds with a bare `zig build-lib host.zig`. ABI modeled on
//! src/default_platform/c_runtime.zig + test/fx-open (entry + List args).

const builtin = @import("builtin");

const seamless_slice_tag: usize = 1;

const c = struct {
    extern fn malloc(size: usize) ?*anyopaque;
    extern fn free(ptr: ?*anyopaque) void;
    extern fn exit(code: i32) noreturn;
    extern fn write(fd: i32, buf: [*]const u8, len: usize) isize;
    extern fn getenv(name: [*:0]const u8) ?[*:0]u8;
    extern fn fopen(path: [*:0]const u8, mode: [*:0]const u8) ?*anyopaque;
    extern fn fread(ptr: [*]u8, size: usize, n: usize, stream: *anyopaque) usize;
    extern fn fseek(stream: *anyopaque, off: c_long, whence: c_int) c_int;
    extern fn ftell(stream: *anyopaque) c_long;
    extern fn fclose(stream: *anyopaque) c_int;
};

const AllocationHeader = extern struct { raw: [*]u8, len: usize };

const AchukStr = extern struct {
    bytes: ?[*]u8,
    capacity_or_alloc_ptr: usize,
    length: usize,

    fn isSmallStr(self: AchukStr) bool {
        return @as(isize, @bitCast(self.length)) < 0;
    }
    fn isSeamlessSlice(self: AchukStr) bool {
        return !self.isSmallStr() and (self.capacity_or_alloc_ptr & seamless_slice_tag) == seamless_slice_tag;
    }
    fn len(self: AchukStr) usize {
        if (self.isSmallStr()) {
            const raw: *const [@sizeOf(AchukStr)]u8 = @ptrCast(&self);
            return raw.*[@sizeOf(AchukStr) - 1] ^ 0b1000_0000;
        }
        return self.length;
    }
    fn allocationPtr(self: AchukStr) ?[*]u8 {
        if (self.isSmallStr()) return null;
        if (self.isSeamlessSlice()) return @ptrFromInt(self.capacity_or_alloc_ptr & ~seamless_slice_tag);
        return self.bytes;
    }
    fn asSlice(self: *const AchukStr) []const u8 {
        const ptr: [*]const u8 = if (self.isSmallStr()) @ptrCast(self) else @ptrCast(self.bytes.?);
        return ptr[0..self.len()];
    }
    fn decref(self: *AchukStr) void {
        const data = self.allocationPtr() orelse return;
        const refcount_ptr: *isize = @ptrCast(@alignCast(data - @sizeOf(usize)));
        if (refcount_ptr.* == 0) return;
        const last = @atomicRmw(isize, refcount_ptr, .Sub, 1, .monotonic);
        if (last == 1) roc_dealloc(data - @sizeOf(usize), @alignOf(usize));
    }
    /// Build a AchukStr that Achuk owns (refcount 1) from a byte slice.
    fn fromSlice(slice: []const u8) AchukStr {
        if (slice.len < @sizeOf(AchukStr)) {
            // small-string: payload inline, last byte = len | 0x80
            var result: AchukStr = .{ .bytes = null, .capacity_or_alloc_ptr = 0, .length = 0 };
            const out: [*]u8 = @ptrCast(&result);
            for (0..slice.len) |i| out[i] = slice[i];
            out[@sizeOf(AchukStr) - 1] = @intCast(slice.len | 0b1000_0000);
            return result;
        }
        // big: [refcount:usize][data...]; bytes points past the refcount word.
        const user_any = roc_alloc(@sizeOf(usize) + slice.len, @alignOf(usize)) orelse
            return .{ .bytes = null, .capacity_or_alloc_ptr = 0, .length = @bitCast(@as(isize, -1)) };
        const user: [*]u8 = @ptrCast(user_any);
        const rc: *isize = @ptrCast(@alignCast(user));
        rc.* = 1;
        const data = user + @sizeOf(usize);
        for (0..slice.len) |i| data[i] = slice[i];
        return .{ .bytes = data, .capacity_or_alloc_ptr = slice.len << 1, .length = slice.len };
    }
};

/// List layout: { bytes, length, capacity_or_alloc_ptr } (note field order
/// differs from AchukStr).
const AchukList = extern struct {
    bytes: ?[*]u8,
    length: usize,
    capacity_or_alloc_ptr: usize,
    fn empty() AchukList {
        return .{ .bytes = null, .length = 0, .capacity_or_alloc_ptr = 0 };
    }
};

// --- the app entry point -------------------------------------------------
// main_for_host! : List(Str) => I32 (the Try is collapsed to i32 in main.roc)
extern fn roc_main(args: AchukList) callconv(.c) i32;

export fn main() callconv(.c) c_int {
    // Args unused by the sys demo; pass an empty List(Str).
    return roc_main(AchukList.empty());
}

// --- hosted effects ------------------------------------------------------

export fn roc_stdout_line(str: AchukStr) callconv(.c) void {
    var owned = str;
    writeAll(1, owned.asSlice());
    writeAll(1, "\n");
    owned.decref();
}

export fn roc_env_get(name: AchukStr) callconv(.c) AchukStr {
    var owned = name;
    var buf: [256]u8 = undefined;
    const n = owned.asSlice();
    const nlen = @min(n.len, buf.len - 1);
    for (0..nlen) |i| buf[i] = n[i];
    buf[nlen] = 0;
    owned.decref();
    const val = c.getenv(@ptrCast(&buf)) orelse return AchukStr.fromSlice("");
    var vlen: usize = 0;
    while (val[vlen] != 0) : (vlen += 1) {}
    return AchukStr.fromSlice(val[0..vlen]);
}

export fn roc_file_read(path: AchukStr) callconv(.c) AchukStr {
    var owned = path;
    var pbuf: [1024]u8 = undefined;
    const p = owned.asSlice();
    const plen = @min(p.len, pbuf.len - 1);
    for (0..plen) |i| pbuf[i] = p[i];
    pbuf[plen] = 0;
    owned.decref();

    const f = c.fopen(@ptrCast(&pbuf), "rb") orelse return AchukStr.fromSlice("");
    defer _ = c.fclose(f);
    if (c.fseek(f, 0, 2) != 0) return AchukStr.fromSlice(""); // SEEK_END
    const size = c.ftell(f);
    if (size <= 0) return AchukStr.fromSlice("");
    _ = c.fseek(f, 0, 0); // SEEK_SET
    const usize_size: usize = @intCast(size);
    const heap_any = roc_alloc(usize_size, @alignOf(usize)) orelse return AchukStr.fromSlice("");
    const heap: [*]u8 = @ptrCast(heap_any);
    const got = c.fread(heap, 1, usize_size, f);
    const result = AchukStr.fromSlice(heap[0..got]);
    roc_dealloc(heap_any, @alignOf(usize));
    return result;
}

// --- runtime symbols (from c_runtime.zig) --------------------------------

export fn roc_dbg(bytes: [*]const u8, len: usize) callconv(.c) void {
    writeAll(2, bytes[0..len]);
    writeAll(2, "\n");
}
export fn roc_expect_failed(bytes: [*]const u8, len: usize) callconv(.c) noreturn {
    writeAll(2, "Achuk expect failed: ");
    writeAll(2, bytes[0..len]);
    writeAll(2, "\n");
    c.exit(1);
}
export fn roc_crashed(bytes: [*]const u8, len: usize) callconv(.c) noreturn {
    writeAll(2, "Achuk crashed: ");
    writeAll(2, bytes[0..len]);
    writeAll(2, "\n");
    c.exit(1);
}
export fn roc_alloc(length: usize, alignment: usize) callconv(.c) ?*anyopaque {
    const a = normalizedAlignment(alignment);
    const raw_any = c.malloc(length + a + @sizeOf(AllocationHeader)) orelse return null;
    const raw: [*]u8 = @ptrCast(raw_any);
    const user_addr = alignForward(@intFromPtr(raw) + @sizeOf(AllocationHeader), a);
    const user: [*]u8 = @ptrFromInt(user_addr);
    allocationHeader(user).* = .{ .raw = raw, .len = length };
    return @ptrCast(user);
}
export fn roc_realloc(ptr: *anyopaque, new_length: usize, alignment: usize) callconv(.c) ?*anyopaque {
    const old_user: [*]u8 = @ptrCast(ptr);
    const old = allocationHeader(old_user).*;
    const new_ptr = roc_alloc(new_length, alignment) orelse return null;
    const new_user: [*]u8 = @ptrCast(new_ptr);
    const copy = @min(old.len, new_length);
    for (0..copy) |i| new_user[i] = old_user[i];
    roc_dealloc(ptr, alignment);
    return new_ptr;
}
export fn roc_dealloc(ptr: *anyopaque, _: usize) callconv(.c) void {
    const user: [*]u8 = @ptrCast(ptr);
    c.free(@ptrCast(allocationHeader(user).raw));
}
fn allocationHeader(user: [*]u8) *AllocationHeader {
    return @ptrCast(@alignCast(user - @sizeOf(AllocationHeader)));
}
fn normalizedAlignment(alignment: usize) usize {
    return @max(alignment, @alignOf(usize));
}
fn alignForward(value: usize, alignment: usize) usize {
    return (value + alignment - 1) & ~(alignment - 1);
}
fn writeAll(fd: i32, bytes: []const u8) void {
    var remaining = bytes;
    while (remaining.len != 0) {
        const written = c.write(fd, remaining.ptr, remaining.len);
        if (written <= 0) return;
        remaining = remaining[@intCast(written)..];
    }
}
