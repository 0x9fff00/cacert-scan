root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
export SCAN="$root/target/release/cacert-scan --roots $root/patterns/roots.pem --watch $root/patterns/watch.pem"
