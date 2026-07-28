import type { Meta, StoryObj } from "@storybook/nextjs-vite";
import Navbar from "@/components/Navbar";

const meta: Meta<typeof Navbar> = {
  title: "Layout/Navbar",
  component: Navbar,
  parameters: {
    layout: "fullscreen",
  },
};

export default meta;
type Story = StoryObj<typeof Navbar>;

export const Default: Story = {};
