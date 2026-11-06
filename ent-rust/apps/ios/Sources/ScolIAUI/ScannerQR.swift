#if os(iOS)
import AVFoundation
import SwiftUI
import UIKit

/// Lecteur de QR code (AVFoundation, aucune dépendance externe).
struct ScannerQR: UIViewControllerRepresentable {
    let surCode: (String) -> Void

    func makeUIViewController(context: Context) -> ControleurScan {
        let c = ControleurScan()
        c.surCode = surCode
        return c
    }

    func updateUIViewController(_ uiViewController: ControleurScan, context: Context) {}
}

final class ControleurScan: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var surCode: ((String) -> Void)?
    private let session = AVCaptureSession()
    private var lu = false

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        view.accessibilityLabel = "Caméra : visez le QR code ScolIA"
        guard let camera = AVCaptureDevice.default(for: .video),
              let entree = try? AVCaptureDeviceInput(device: camera),
              session.canAddInput(entree) else { return }
        session.addInput(entree)
        let sortie = AVCaptureMetadataOutput()
        guard session.canAddOutput(sortie) else { return }
        session.addOutput(sortie)
        sortie.setMetadataObjectsDelegate(self, queue: .main)
        sortie.metadataObjectTypes = [.qr]
        let apercu = AVCaptureVideoPreviewLayer(session: session)
        apercu.frame = view.layer.bounds
        apercu.videoGravity = .resizeAspectFill
        view.layer.addSublayer(apercu)
        DispatchQueue.global(qos: .userInitiated).async { [session] in session.startRunning() }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        session.stopRunning()
    }

    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput objets: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !lu, let o = objets.first as? AVMetadataMachineReadableCodeObject, let texte = o.stringValue else { return }
        lu = true
        session.stopRunning()
        surCode?(texte)
    }
}
#endif
