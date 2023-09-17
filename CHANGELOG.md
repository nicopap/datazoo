# 0.4.0

Remove `From<usize>` bound on `K` in `RawIndexMap`. It was unneeded, because
`Index` trait already can perform this conversion.

# 0.3.0

Make `JaggedArray` generic over the storage kind, by adding the `VS` type
parameter. It makes the type definitively fairly difficult to gork, but it
enables things like subslicing.

# 0.2.0

Initial release.
