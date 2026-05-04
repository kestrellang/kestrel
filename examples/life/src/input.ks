module Life

import Sdl.(Event)

protocol InputManager {
    mutating func getEvent() -> Optional[Event]
}

struct HeadlessInputManager: InputManager {
    var remaining: Int64

    init(iters: Int64) {
        self.remaining = iters;
    }

    mutating func getEvent() -> Optional[Event] {
        if self.remaining == 0 {
            self.remaining = self.remaining - 1;
            return .Some(Event.Quit)
        }
        if self.remaining > 0 {
            self.remaining = self.remaining - 1;
        }
        .None
    }
}
