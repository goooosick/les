TARGET=les_bevy
OUT_DIR=web/pkg

cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir $OUT_DIR --no-modules --no-typescript ./target/wasm32-unknown-unknown/release/$TARGET.wasm
# wasm-opt $OUT_DIR/$TARGET_bg.wasm -O2 --fast-math -o $OUT_DIR/$TARGET_bg.wasm
