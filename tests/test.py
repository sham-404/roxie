import chess

board = chess.Board(
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
)


def qperft(board, depth):
    if depth == 0:
        return 1

    nodes = 0

    if board.is_check():
        moves = list(board.legal_moves)
    else:
        moves = [m for m in board.legal_moves if board.is_capture(m)]

    for move in moves:
        board.push(move)
        nodes += qperft(board, depth - 1)
        board.pop()

    return nodes


print(qperft(board, 5))
