import { act } from 'react-dom/test-utils';
import { createRoot, Root } from 'react-dom/client';
import { FilterPanel } from '../../components/contracts/FilterPanel';

describe('FilterPanel', () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
  });

  test('renders reset button and calls onResetAll when clicked', () => {
    const onResetAll = jest.fn();

    act(() => {
      root.render(
        <FilterPanel
          categories={[{ value: 'DeFi', label: 'DeFi', count: 7 }]}
          selectedCategories={['DeFi']}
          onToggleCategory={jest.fn()}
          onClearCategories={jest.fn()}
          networks={[{ value: 'mainnet', label: 'Mainnet', count: 4 }]}
          selectedNetworks={['mainnet']}
          onToggleNetwork={jest.fn()}
          onClearNetworks={jest.fn()}
          languages={['Rust']}
          selectedLanguages={[]}
          onToggleLanguage={jest.fn()}
          author=""
          onAuthorChange={jest.fn()}
          verifiedOnly={false}
          onVerifiedChange={jest.fn()}
          activeFilterCount={2}
          onResetAll={onResetAll}
        />,
      );
    });

    const resetButton = Array.from(container.querySelectorAll('button')).find(
      (button) => button.textContent?.includes('Reset'),
    );

    expect(container.textContent).toContain('2 active filters');
    expect(resetButton).toBeTruthy();

    act(() => {
      resetButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(onResetAll).toHaveBeenCalledTimes(1);
  });
});
