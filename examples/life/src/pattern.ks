module Life

enum Pattern: Formattable {
    case Glider
    case Blinker
    case Lwss
    case Pulsar
    case GosperGun

    func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        let s = match self {
            .Glider => "GLIDER",
            .Blinker => "BLINKER",
            .Lwss => "LWSS",
            .Pulsar => "PULSAR",
            .GosperGun => "GOSPER GUN"
        };
        writer.append(s);
    }

    func stamp(mutating on grid: Grid, centerX cx: Int64, centerY cy: Int64) {
        match self {
            .Glider => {
                let x = cx - 1; let y = cy - 1;
                grid.setCell(x: x + 1, y: y + 0, alive: true);
                grid.setCell(x: x + 2, y: y + 1, alive: true);
                grid.setCell(x: x + 0, y: y + 2, alive: true);
                grid.setCell(x: x + 1, y: y + 2, alive: true);
                grid.setCell(x: x + 2, y: y + 2, alive: true);
            },
            .Blinker => {
                grid.setCell(x: cx - 1, y: cy, alive: true);
                grid.setCell(x: cx,     y: cy, alive: true);
                grid.setCell(x: cx + 1, y: cy, alive: true);
            },
            .Lwss => {
                let x = cx - 2; let y = cy - 2;
                grid.setCell(x: x + 1, y: y + 0, alive: true);
                grid.setCell(x: x + 4, y: y + 0, alive: true);
                grid.setCell(x: x + 0, y: y + 1, alive: true);
                grid.setCell(x: x + 0, y: y + 2, alive: true);
                grid.setCell(x: x + 4, y: y + 2, alive: true);
                grid.setCell(x: x + 0, y: y + 3, alive: true);
                grid.setCell(x: x + 1, y: y + 3, alive: true);
                grid.setCell(x: x + 2, y: y + 3, alive: true);
                grid.setCell(x: x + 3, y: y + 3, alive: true);
            },
            .Pulsar => {
                let x = cx - 6; let y = cy - 6;
                grid.setCell(x: x + 2, y: y + 0, alive: true);
                grid.setCell(x: x + 3, y: y + 0, alive: true);
                grid.setCell(x: x + 4, y: y + 0, alive: true);
                grid.setCell(x: x + 8, y: y + 0, alive: true);
                grid.setCell(x: x + 9, y: y + 0, alive: true);
                grid.setCell(x: x + 10, y: y + 0, alive: true);
                grid.setCell(x: x + 0, y: y + 2, alive: true);
                grid.setCell(x: x + 5, y: y + 2, alive: true);
                grid.setCell(x: x + 7, y: y + 2, alive: true);
                grid.setCell(x: x + 12, y: y + 2, alive: true);
                grid.setCell(x: x + 0, y: y + 3, alive: true);
                grid.setCell(x: x + 5, y: y + 3, alive: true);
                grid.setCell(x: x + 7, y: y + 3, alive: true);
                grid.setCell(x: x + 12, y: y + 3, alive: true);
                grid.setCell(x: x + 0, y: y + 4, alive: true);
                grid.setCell(x: x + 5, y: y + 4, alive: true);
                grid.setCell(x: x + 7, y: y + 4, alive: true);
                grid.setCell(x: x + 12, y: y + 4, alive: true);
                grid.setCell(x: x + 2, y: y + 5, alive: true);
                grid.setCell(x: x + 3, y: y + 5, alive: true);
                grid.setCell(x: x + 4, y: y + 5, alive: true);
                grid.setCell(x: x + 8, y: y + 5, alive: true);
                grid.setCell(x: x + 9, y: y + 5, alive: true);
                grid.setCell(x: x + 10, y: y + 5, alive: true);
                grid.setCell(x: x + 2, y: y + 7, alive: true);
                grid.setCell(x: x + 3, y: y + 7, alive: true);
                grid.setCell(x: x + 4, y: y + 7, alive: true);
                grid.setCell(x: x + 8, y: y + 7, alive: true);
                grid.setCell(x: x + 9, y: y + 7, alive: true);
                grid.setCell(x: x + 10, y: y + 7, alive: true);
                grid.setCell(x: x + 0, y: y + 8, alive: true);
                grid.setCell(x: x + 5, y: y + 8, alive: true);
                grid.setCell(x: x + 7, y: y + 8, alive: true);
                grid.setCell(x: x + 12, y: y + 8, alive: true);
                grid.setCell(x: x + 0, y: y + 9, alive: true);
                grid.setCell(x: x + 5, y: y + 9, alive: true);
                grid.setCell(x: x + 7, y: y + 9, alive: true);
                grid.setCell(x: x + 12, y: y + 9, alive: true);
                grid.setCell(x: x + 0, y: y + 10, alive: true);
                grid.setCell(x: x + 5, y: y + 10, alive: true);
                grid.setCell(x: x + 7, y: y + 10, alive: true);
                grid.setCell(x: x + 12, y: y + 10, alive: true);
                grid.setCell(x: x + 2, y: y + 12, alive: true);
                grid.setCell(x: x + 3, y: y + 12, alive: true);
                grid.setCell(x: x + 4, y: y + 12, alive: true);
                grid.setCell(x: x + 8, y: y + 12, alive: true);
                grid.setCell(x: x + 9, y: y + 12, alive: true);
                grid.setCell(x: x + 10, y: y + 12, alive: true);
            },
            .GosperGun => {
                let x = cx - 18; let y = cy - 4;
                grid.setCell(x: x + 0, y: y + 4, alive: true);
                grid.setCell(x: x + 1, y: y + 4, alive: true);
                grid.setCell(x: x + 0, y: y + 5, alive: true);
                grid.setCell(x: x + 1, y: y + 5, alive: true);
                grid.setCell(x: x + 10, y: y + 4, alive: true);
                grid.setCell(x: x + 10, y: y + 5, alive: true);
                grid.setCell(x: x + 10, y: y + 6, alive: true);
                grid.setCell(x: x + 11, y: y + 3, alive: true);
                grid.setCell(x: x + 11, y: y + 7, alive: true);
                grid.setCell(x: x + 12, y: y + 2, alive: true);
                grid.setCell(x: x + 12, y: y + 8, alive: true);
                grid.setCell(x: x + 13, y: y + 2, alive: true);
                grid.setCell(x: x + 13, y: y + 8, alive: true);
                grid.setCell(x: x + 14, y: y + 5, alive: true);
                grid.setCell(x: x + 15, y: y + 3, alive: true);
                grid.setCell(x: x + 15, y: y + 7, alive: true);
                grid.setCell(x: x + 16, y: y + 4, alive: true);
                grid.setCell(x: x + 16, y: y + 5, alive: true);
                grid.setCell(x: x + 16, y: y + 6, alive: true);
                grid.setCell(x: x + 17, y: y + 5, alive: true);
                grid.setCell(x: x + 20, y: y + 2, alive: true);
                grid.setCell(x: x + 20, y: y + 3, alive: true);
                grid.setCell(x: x + 20, y: y + 4, alive: true);
                grid.setCell(x: x + 21, y: y + 2, alive: true);
                grid.setCell(x: x + 21, y: y + 3, alive: true);
                grid.setCell(x: x + 21, y: y + 4, alive: true);
                grid.setCell(x: x + 22, y: y + 1, alive: true);
                grid.setCell(x: x + 22, y: y + 5, alive: true);
                grid.setCell(x: x + 24, y: y + 0, alive: true);
                grid.setCell(x: x + 24, y: y + 1, alive: true);
                grid.setCell(x: x + 24, y: y + 5, alive: true);
                grid.setCell(x: x + 24, y: y + 6, alive: true);
                grid.setCell(x: x + 34, y: y + 2, alive: true);
                grid.setCell(x: x + 34, y: y + 3, alive: true);
                grid.setCell(x: x + 35, y: y + 2, alive: true);
                grid.setCell(x: x + 35, y: y + 3, alive: true);
            }
        }
    }
}
