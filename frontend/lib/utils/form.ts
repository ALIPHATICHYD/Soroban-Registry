/**
 * Helper utilities for form components
 * Provides base styling classes and utilities for form controls
 */

import { FORM_INPUT, FORM_TEXTAREA, FORM_SELECT } from "@/constants/tailwind";

/**
 * Get the base form control ID
 * Falls back to name if id is not provided
 */
export function getFormControlId(
  id: string | undefined,
  name: string | undefined,
): string | undefined {
  return id || name;
}

/**
 * Build className for form input control
 */
export function getInputClassName(additionalClass?: string): string {
  return additionalClass ? `${FORM_INPUT} ${additionalClass}` : FORM_INPUT;
}

/**
 * Build className for form textarea control
 */
export function getTextareaClassName(additionalClass?: string): string {
  return additionalClass
    ? `${FORM_TEXTAREA} ${additionalClass}`
    : FORM_TEXTAREA;
}

/**
 * Build className for form select control
 */
export function getSelectClassName(additionalClass?: string): string {
  return additionalClass ? `${FORM_SELECT} ${additionalClass}` : FORM_SELECT;
}

/**
 * Get aria-describedby value for form error
 */
export function getAriaDescribedBy(
  id: string | undefined,
  hasError: boolean,
): string | undefined {
  return hasError && id ? `${id}-error` : undefined;
}
