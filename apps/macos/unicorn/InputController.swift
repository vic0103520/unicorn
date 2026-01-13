import Cocoa
import InputMethodKit
import os

@objc(InputController) public class InputController: IMKInputController {

    private let logger = Logger(
        subsystem: "Vic-Shih.inputmethod.unicorn", category: "InputController")
    private var candidatesVisible = false
    private var currentBuffer: String = ""
    private var selectionIndex: Int = 0
    private var firstVisibleCandidateIndex: Int = 0
    private let pageSize = 9

    public required override init!(server: IMKServer!, delegate: Any!, client: Any!) {
        super.init(server: server, delegate: delegate, client: client)
    }

    lazy var engine: Engine? = {
        guard let path = Bundle.main.path(forResource: "keymap", ofType: "json") else {
            logger.error("Unicorn Error: keymap.json not found in bundle.")
            return nil
        }
        do {
            return try Engine.newFromPath(path: path)
        } catch {
            logger.error("Unicorn Error: Failed to load engine: \(error.localizedDescription)")
            return nil
        }
    }()

    // MARK: - Lifecycle

    public override func deactivateServer(_ sender: Any!) {
        engine?.deactivate()
        candidatesWindow?.hide()
        candidatesVisible = false
        currentBuffer = ""
        selectionIndex = 0
        firstVisibleCandidateIndex = 0
        super.deactivateServer(sender)
    }

    // MARK: - Event Handling
    @objc(handleEvent:client:)
    public override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event = event else { return false }
        
        logger.debug("Handle event: type=\(event.type.rawValue), keyCode=\(event.keyCode), modifiers=\(event.modifierFlags.rawValue)")

        // Explicitly let the system handle modifier key changes (Caps Lock switching)
        if event.type == .flagsChanged {
            logger.debug("Flags changed event detected. Returning false to allow system handling.")
            return false
        }

        if event.type == .keyDown {
            if let chars = event.characters, chars == "\\" {
                let modifiers = event.modifierFlags.intersection([.command, .control, .option])
                if modifiers.isEmpty {
                    return self.inputText(chars, client: sender)
                }
            }

            if event.keyCode == 51 {
                if let engine = self.engine {
                    let actions = engine.processKey(charCode: 0x08)
                    if actions.isEmpty {
                         // Should not happen based on core logic
                    } else if case .reject = actions[0] {
                        // Inactive, fall through
                    } else {
                        guard let client = sender as? IMKTextInput else { return true }
                        for action in actions {
                            switch action {
                            case .updateComposition(let text):
                                handleUpdate(text, showCandidates: false, client: client)
                            case .showCandidates(let text):
                                handleUpdate(text, showCandidates: true, client: client)
                            case .commit(let text):
                                commitText(text, client: client)
                            case .reject:
                                break
                            }
                        }
                        return true
                    }
                }
            }

            if candidatesVisible {
                let candidates = engine?.getCandidates() ?? []
                let number_of_candidates = candidates.count

                if event.keyCode == 125 {  // Down
                    if selectionIndex < number_of_candidates - 1 {
                        selectionIndex += 1
                        if selectionIndex >= firstVisibleCandidateIndex + pageSize {
                            firstVisibleCandidateIndex = selectionIndex - pageSize + 1
                        }
                        engine?.selectCandidate(index: UInt32(selectionIndex))
                        candidatesWindow?.moveDown(sender)
                    }
                    return true
                }
                if event.keyCode == 126 {  // Up
                    if selectionIndex > 0 {
                        selectionIndex -= 1
                        if selectionIndex < firstVisibleCandidateIndex {
                            firstVisibleCandidateIndex = selectionIndex
                        }
                        engine?.selectCandidate(index: UInt32(selectionIndex))
                        candidatesWindow?.moveUp(sender)
                    }
                    return true
                }
                if event.keyCode == 124 {  // Right (Page Down)
                    let nextFirst = firstVisibleCandidateIndex + pageSize
                    if nextFirst < number_of_candidates {
                        firstVisibleCandidateIndex = nextFirst
                        selectionIndex = firstVisibleCandidateIndex
                    } else {
                        selectionIndex = number_of_candidates - 1
                        firstVisibleCandidateIndex = max(0, number_of_candidates - pageSize)
                    }
                    engine?.selectCandidate(index: UInt32(selectionIndex))
                    candidatesWindow?.moveRight(sender)
                    return true
                }
                if event.keyCode == 123 {  // Left (Page Up)
                    firstVisibleCandidateIndex -= pageSize
                    if firstVisibleCandidateIndex < 0 {
                        firstVisibleCandidateIndex = 0
                    }
                    selectionIndex = firstVisibleCandidateIndex
                    engine?.selectCandidate(index: UInt32(selectionIndex))
                    candidatesWindow?.moveLeft(sender)
                    return true
                }
                if event.keyCode == 36 || event.keyCode == 76 {  // Enter
                    if selectionIndex < candidates.count {
                        commitText(candidates[selectionIndex], client: sender as? IMKTextInput)
                        return false
                    }
                }
            }

            if let chars = event.characters {
                let modifiers = event.modifierFlags.intersection([.command, .control, .option])
                if modifiers.isEmpty {
                    return self.inputText(chars, client: sender)
                }
            }

            return false
        }

        return super.handle(event, client: sender)
    }

    public override func inputText(_ string: String!, client sender: Any!) -> Bool {
        guard let client = sender as? IMKTextInput else { return false }
        guard let engine = self.engine else { return false }
        guard let string = string else { return false }

        for scalar in string.unicodeScalars {
            let actions = engine.processKey(charCode: scalar.value)

            for action in actions {
                switch action {
                case .reject:
                    if string == "\\", self.currentBuffer.isEmpty {
                        self.currentBuffer = "\\"
                        self.candidatesVisible = false
                        self.selectionIndex = 0
                        client.setMarkedText(
                            "\\",
                            selectionRange: NSRange(location: 1, length: 0),
                            replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
                        return true
                    }

                    if candidatesVisible {
                        if let number = Int(string), number >= 1, number <= 9 {
                            let candidates = engine.getCandidates()
                            let index = number - 1
                            if index < candidates.count {
                                commitText(candidates[index], client: client)
                                return true
                            }
                        }
                        if string == " " {
                            if selectionIndex < engine.getCandidates().count {
                                commitText(engine.getCandidates()[selectionIndex], client: client)
                                return true
                            }
                        }
                    }

                    let candidates = engine.getCandidates()
                    if let firstCandidate = candidates.first {
                        client.insertText(
                            firstCandidate,
                            replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
                    } else {
                        if !self.currentBuffer.isEmpty {
                            client.insertText(
                                self.currentBuffer,
                                replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
                        }
                    }

                    engine.deactivate()
                    candidatesWindow?.hide()
                    candidatesVisible = false
                    currentBuffer = ""
                    selectionIndex = 0
                    firstVisibleCandidateIndex = 0
                    return false  // Pass invalid char to system

                case .updateComposition(let text):
                    handleUpdate(text, showCandidates: false, client: client)

                case .showCandidates(let text):
                    handleUpdate(text, showCandidates: true, client: client)

                case .commit(let text):
                    commitText(text, client: client, shouldDeactivate: false)
                }
            }
        }
        return true
    }

    private func handleUpdate(_ text: String, showCandidates: Bool, client: IMKTextInput) {
        self.currentBuffer = text
        self.candidatesVisible = showCandidates
        self.selectionIndex = 0  // Reset selection when candidates change
        self.firstVisibleCandidateIndex = 0
        self.engine?.selectCandidate(index: 0)

        client.setMarkedText(
            text,
            selectionRange: NSRange(location: text.count, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound))

        if showCandidates {
            candidatesWindow?.update()
            candidatesWindow?.show()
        } else {
            candidatesWindow?.hide()
        }
    }

    private func commitText(_ text: String, client: IMKTextInput?, shouldDeactivate: Bool = true) {
        client?.insertText(
            text, replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
        candidatesWindow?.hide()
        candidatesVisible = false
        currentBuffer = ""
        selectionIndex = 0
        firstVisibleCandidateIndex = 0
        if shouldDeactivate {
            engine?.deactivate()
        }
    }

    public override func candidates(_ sender: Any!) -> [Any]! {
        return engine?.getCandidates() ?? []
    }

    public override func candidateSelected(_ candidateString: NSAttributedString!) {
        guard let client = client() else { return }
        commitText(candidateString.string, client: client)
    }
}
