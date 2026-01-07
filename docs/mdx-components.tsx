import {useMDXComponents as getDocsMDXComponents} from 'nextra-theme-docs'
import {ComponentType} from "react";

const docsComponents = getDocsMDXComponents()

export function useMDXComponents(components?: Record<string, ComponentType>) {
    return {
        ...docsComponents,
        ...components
    }
}
