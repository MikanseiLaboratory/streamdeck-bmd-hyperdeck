import "react";

declare module "react" {
  interface HTMLAttributes<T> {
    type?: string;
  }
}
