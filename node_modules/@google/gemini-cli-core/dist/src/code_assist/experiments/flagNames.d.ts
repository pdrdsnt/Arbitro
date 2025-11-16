/**
 * @license
 * Copyright 2025 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */
export declare const ExperimentFlags: {
    readonly CONTEXT_COMPRESSION_THRESHOLD: "GeminiCLIContextCompression__threshold_fraction";
    readonly USER_CACHING: "GcliUserCaching__user_caching";
};
export type ExperimentFlagName = (typeof ExperimentFlags)[keyof typeof ExperimentFlags];
