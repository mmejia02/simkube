package main

import (
	"fmt"
	"os"

	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	"k8s.io/client-go/scale/scheme"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/config"

	"simkube/cli/cmd"
	simkubev1 "simkube/lib/go/api/v1"
)

//nolint:gochecknoglobals
var simulationScheme = runtime.NewScheme()

func main() {
	k8sClient, err := client.New(config.GetConfigOrDie(), client.Options{Scheme: simulationScheme})
	if err != nil {
		fmt.Printf("could not construct Kubernetes client: %v\n", err)
		os.Exit(1)
	}

	if err := cmd.Root(k8sClient).Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

//nolint:gochecknoinits // generated by kubebuilder
func init() {
	utilruntime.Must(scheme.AddToScheme(simulationScheme))
	utilruntime.Must(simkubev1.AddToScheme(simulationScheme))
}