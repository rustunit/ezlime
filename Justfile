

allow:
    direnv allow

check:
    cargo c
    cargo clippy
    cargo t
    cargo sort --check
    cargo fmt --check

fix:
    cargo clippy --fix
    cargo sort
    cargo fmt

test-create:
	echo '{"url":"https://www.rustunit.com"}' | xh POST "localhost:8080/link/create" content-type:application/json Authorization:a

test-get:
    xh "localhost:8080/as9sud"
