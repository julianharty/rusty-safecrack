# Rusty Safecrack
This is a simple exercise to create pairwise tests and run them against an instance of the safecrack example website.

The rust code is more mature than the typescript.

## Typescript
Run the following commands in the typescript directory:
```bash
cd typescript
npm install axios cheerio                     
npm install --save-dev @types/node ts-node typescript
```
Then you can run commands such as: 
```
node --experimental-strip-types safecracker.ts http://localhost:5004 --attempts 50 --delay 20
```

## Rust
```
cargo run --bin safecracker http://127.0.0.1:5004 --debug -- --delay=15  && cat safecrack_report.md
```
