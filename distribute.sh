cargo b --release --bin device --features "bt"
cargo b --release --bin device --target aarch64-unknown-linux-gnu

mkdir -p eval-prep/keys
cp -r instances eval-prep/instances

cp keys/runner.cert.der eval-prep/keys

# socorro / gozo (nucs)
cp target/release/device eval-prep/

cp keys/socorro.cert.der eval-prep/keys/self.cert.der
cp keys/socorro.key.der eval-prep/keys/self.key.der
scp -r eval-prep/. netlab@socorro:~/evaluation
# ssh netlab@socorro 'cd evaluation; sudo pkill device; sudo ./device &'

cp keys/gozo.cert.der eval-prep/keys/self.cert.der
cp keys/gozo.key.der eval-prep/keys/self.key.der
scp -r eval-prep/. netlab@gozo:~/evaluation
# ssh netlab@gozo 'cd evaluation; sudo pkill device; sudo ./device &'

# rpi
cp target/aarch64-unknown-linux-gnu/release/device eval-prep/
cp keys/raspberrypi.cert.der eval-prep/keys/self.cert.der
cp keys/raspberrypi.key.der eval-prep/keys/self.key.der
scp -r eval-prep/. trusded@raspberrypi:~/evaluation
# ssh trusded@raspberrypi 'cd evaluation; sudo pkill device; sudo ./device &'
