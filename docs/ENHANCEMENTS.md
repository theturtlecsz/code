# Enhancement Requests

## UI/UX Improvements

### Model Indicator Display (2025-11-20)

**Request**: Show current model more prominently during usage

**Proposed improvements**:
1. **Status bar**: Persistent model display (always visible)
   - Location: Bottom status bar or top bar
   - Format: `Model: claude-sonnet-4.5 | Effort: medium`

2. **Prompt prefix**: Show model before each input
   - Format: `[claude-sonnet-4.5] > What's 2+2?`
   - Helps during testing and multi-model workflows

**Current behavior**: Model change shows in chat history (chatwidget/mod.rs:10061)

**Priority**: Low (quality of life improvement)
**Complexity**: Low (~30 min implementation)
**Related**: SPEC-KIT-952 testing identified the need during multi-model testing

**Implementation notes**:
- Status bar: Modify `bottom_pane` status display
- Prompt prefix: Add to input rendering in `chatwidget`
- Consider color coding by provider (Claude=purple, Gemini=blue, GPT=green)

---
