# NOTE: only for test
# NOTE: run this script in root mode

cargo build

MNT_POINT="target/mnt"

umount $MNT_POINT
if [ ! -d "$MNT_POINT" ];then
    mkdir $MNT_POINT
fi

sleep 1
RUST_LOG=trace ./target/debug/memfs $MNT_POINT
