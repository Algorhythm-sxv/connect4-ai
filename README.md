# connect4-ai
A game solver for connect 4 with a CLI frontend.

It cannot be beaten unless its oppenent plays first and plays perfectly!

## Usage
`cargo run --release` to play a game.

The AI code exists in a library separate to the CLI frontend, so it can be embedded in other projects

## Details
This agent uses a classical game-tree search with various optimisations:
- alpha-beta pruning
- iterative deepening
- bitboard representation
- transposition tables
- an 'opening book' of all positions with exactly 12 tiles

On my machine (AMD Ryzen 5 1600) any position can be solved in under 1 second.

## Future improvements
- Add documentation
- Add WASM target for the web
- Multithreading with the Lazy SMP technique
