import type { Meta, StoryObj } from "@storybook/nextjs-vite";
import DependencyGraph from "@/components/DependencyGraph";

const meta: Meta<typeof DependencyGraph> = {
  title: "Components/DependencyGraph",
  component: DependencyGraph,
};

export default meta;
type Story = StoryObj<typeof DependencyGraph>;

export const Default: Story = {
  args: {
    nodes: [
      {
        id: "1",
        contract_id: "C1",
        name: "Contract A",
        network: "mainnet",
        is_verified: true,
        tags: [],
      },
      {
        id: "2",
        contract_id: "C2",
        name: "Contract B",
        network: "testnet",
        is_verified: false,
        tags: [],
      },
    ],
    edges: [{ source: "1", target: "2", dependency_type: "calls" }],
  },
};
