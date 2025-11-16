import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { RadioButtonSelect } from './shared/RadioButtonSelect.js';
import { theme } from '../semantic-colors.js';
export function ProQuotaDialog({ fallbackModel, onChoice, }) {
    const items = [
        {
            label: 'Try again later',
            value: 'retry_later',
            key: 'retry_later',
        },
        {
            label: `Switch to ${fallbackModel} for the rest of this session`,
            value: 'retry',
            key: 'retry',
        },
    ];
    const handleSelect = (choice) => {
        onChoice(choice);
    };
    return (_jsxs(Box, { borderStyle: "round", flexDirection: "column", paddingX: 1, children: [_jsx(Box, { marginTop: 1, marginBottom: 1, children: _jsx(RadioButtonSelect, { items: items, initialIndex: 1, onSelect: handleSelect }) }), _jsx(Text, { color: theme.text.primary, children: "Note: You can always use /model to select a different option." })] }));
}
//# sourceMappingURL=ProQuotaDialog.js.map