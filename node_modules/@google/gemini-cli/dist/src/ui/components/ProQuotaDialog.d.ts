/**
 * @license
 * Copyright 2025 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */
import type React from 'react';
interface ProQuotaDialogProps {
    fallbackModel: string;
    onChoice: (choice: 'retry_later' | 'retry') => void;
}
export declare function ProQuotaDialog({ fallbackModel, onChoice, }: ProQuotaDialogProps): React.JSX.Element;
export {};
