import { test, expect } from '@playwright/test';
import { geometryChecks } from 'linen-ui/tooling/flyout-geometry.mjs';
import { mockState } from '../../src/state.js';

geometryChecks(test, expect, { ...mockState(), language: 'en' });
